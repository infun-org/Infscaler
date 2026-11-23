//! Face crop, alignment, ONNX restoration, and feathered paste-back.
use anyhow::{anyhow, bail, Context, Result};
use image::{Rgba, RgbaImage};
use ndarray::Array4;
use ort::{session::Session, value::Tensor};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct FaceBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub angle: f32,
}

/// Paste a 512-square restored face into its detected rectangle. The mask is
/// an inset rounded-square mask with a cosine feather; coords clip to edges.
#[allow(dead_code)]
pub fn paste_face(base: &mut RgbaImage, restored: &RgbaImage, rect: FaceBox, strength: f32) {
    let strength = strength.clamp(0.0, 1.0);
    if strength == 0.0 || rect.width == 0 || rect.height == 0 {
        return;
    }
    let (w, h) = base.dimensions();
    let x0 = rect.x.min(w);
    let y0 = rect.y.min(h);
    let x1 = rect.x.saturating_add(rect.width).min(w);
    let y1 = rect.y.saturating_add(rect.height).min(h);
    let feather = 0.16f32;
    for y in y0..y1 {
        for x in x0..x1 {
            let u = (x - rect.x) as f32 / rect.width as f32;
            let v = (y - rect.y) as f32 / rect.height as f32;
            let dx = (u - 0.5).abs() / 0.5;
            let dy = (v - 0.5).abs() / 0.5;
            let edge = dx.max(dy);
            let t = ((1.0 - edge) / feather).clamp(0.0, 1.0);
            let mask = strength * (t * t * (3.0 - 2.0 * t));
            if mask <= 0.0 {
                continue;
            }
            let sx = ((u * restored.width() as f32).floor() as u32).min(restored.width() - 1);
            let sy = ((v * restored.height() as f32).floor() as u32).min(restored.height() - 1);
            let a = *base.get_pixel(x, y);
            let b = *restored.get_pixel(sx, sy);
            base.put_pixel(
                x,
                y,
                Rgba([
                    blend(a[0], b[0], mask),
                    blend(a[1], b[1], mask),
                    blend(a[2], b[2], mask),
                    a[3],
                ]),
            );
        }
    }
}
fn blend(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 * (1.0 - t) + b as f32 * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Run YuNet's three stride heads on a padded 640x640 image (BGR tensor). The exported
/// OpenCV Zoo graph returns cls/obj/bbox/kps tensors for strides 8,16,32.
pub fn detect(path: &Path, image: &RgbaImage) -> Result<Vec<FaceBox>> {
    let mut s = Session::builder(crate::ort_backend::environment()?)
        .map_err(|e| anyhow!("face session builder: {e}"))?
        .commit_from_file(path)
        .map_err(|e| anyhow!("load face detector {}: {e}", path.display()))?;
    let (w, h) = image.dimensions();
    let scale = 640.0 / (w.max(h) as f32);
    let rw = ((w as f32 * scale).round() as u32).max(1);
    let rh = ((h as f32 * scale).round() as u32).max(1);
    let resized = image::imageops::resize(image, rw, rh, image::imageops::FilterType::Triangle);
    let mut input = vec![0f32; 3 * 640 * 640];
    for y in 0..rh {
        for x in 0..rw {
            let p = resized.get_pixel(x, y);
            let i = (y * 640 + x) as usize;
            input[i] = p[2] as f32;
            input[640 * 640 + i] = p[1] as f32;
            input[2 * 640 * 640 + i] = p[0] as f32;
        }
    }
    let tensor = Tensor::from_array(Array4::from_shape_vec((1, 3, 640, 640), input)?)?;
    let outputs = s
        .run(ort::inputs![tensor])
        .map_err(|e| anyhow!("YuNet inference: {e}"))?;
    let mut candidates = Vec::new();
    for stride in [8usize, 16, 32] {
        let cls = outputs
            .iter()
            .find(|(name, _)| name.starts_with(&format!("cls_{stride}")))
            .context("YuNet cls head missing")?;
        let obj = outputs
            .iter()
            .find(|(name, _)| name.starts_with(&format!("obj_{stride}")))
            .context("YuNet obj head missing")?;
        let bbox = outputs
            .iter()
            .find(|(name, _)| name.starts_with(&format!("bbox_{stride}")))
            .context("YuNet bbox head missing")?;
        let kps = outputs
            .iter()
            .find(|(name, _)| name.starts_with(&format!("kps_{stride}")))
            .context("YuNet landmarks missing")?;
        let c = cls.1.try_extract_array::<f32>()?;
        let o = obj.1.try_extract_array::<f32>()?;
        let b = bbox.1.try_extract_array::<f32>()?;
        let k = kps.1.try_extract_array::<f32>()?;
        let side = 640 / stride;
        for gy in 0..side {
            for gx in 0..side {
                let i = gy * side + gx;
                let score = c[[0, i, 0]] * o[[0, i, 0]];
                if score < 0.45 {
                    continue;
                }
                let cx = (gx as f32 + 0.5) * stride as f32;
                let cy = (gy as f32 + 0.5) * stride as f32;
                let left = cx - b[[0, i, 0]] * stride as f32;
                let top = cy - b[[0, i, 1]] * stride as f32;
                let fw = b[[0, i, 2]].exp() * stride as f32;
                let fh = b[[0, i, 3]].exp() * stride as f32;
                let inv = 1.0 / scale;
                let x = (left * inv).clamp(0.0, w as f32);
                let y = (top * inv).clamp(0.0, h as f32);
                let x2 = ((left + fw) * inv).clamp(0.0, w as f32);
                let y2 = ((top + fh) * inv).clamp(0.0, h as f32);
                let eye0x = (gx as f32 * stride as f32 + k[[0, i, 0]] * stride as f32) * inv;
                let eye0y = (gy as f32 * stride as f32 + k[[0, i, 1]] * stride as f32) * inv;
                let eye1x = (gx as f32 * stride as f32 + k[[0, i, 2]] * stride as f32) * inv;
                let eye1y = (gy as f32 * stride as f32 + k[[0, i, 3]] * stride as f32) * inv;
                // YuNet can label animal muzzles as faces. Reject candidates
                // whose eye landmarks fall well outside the predicted box.
                let face_top = top * inv;
                let face_h = fh * inv;
                let face_left = left * inv;
                let face_w = fw * inv;
                if eye0y < face_top - face_h * 0.12
                    || eye1y < face_top - face_h * 0.12
                    || eye0x < face_left - face_w * 0.2
                    || eye1x < face_left - face_w * 0.2
                    || eye0x > face_left + face_w * 1.2
                    || eye1x > face_left + face_w * 1.2
                {
                    continue;
                }
                let angle = (eye1y - eye0y).atan2(eye1x - eye0x);
                if x2 > x && y2 > y {
                    candidates.push((
                        score,
                        FaceBox {
                            x: x as u32,
                            y: y as u32,
                            width: (x2 - x) as u32,
                            height: (y2 - y) as u32,
                            angle,
                        },
                    ));
                }
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut faces: Vec<FaceBox> = Vec::new();
    for (_, f) in candidates {
        if faces.iter().all(|g| iou(f, *g) < 0.25) {
            faces.push(f);
        }
    }
    Ok(faces)
}
fn iou(a: FaceBox, b: FaceBox) -> f32 {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let r = (a.x + a.width).min(b.x + b.width);
    let d = (a.y + a.height).min(b.y + b.height);
    let inter = r.saturating_sub(x) * d.saturating_sub(y);
    inter as f32 / (a.width * a.height + b.width * b.height - inter).max(1) as f32
}

/// Crop with context, restore the fixed 512 face, then feather it into image.
pub fn restore(path: &Path, image: &mut RgbaImage, faces: &[FaceBox], strength: f32) -> Result<()> {
    if strength <= 0.0 || faces.is_empty() {
        return Ok(());
    }
    let mut s = Session::builder(crate::ort_backend::environment()?)
        .map_err(|e| anyhow!("face session builder: {e}"))?
        .with_intra_threads(4)
        .map_err(|e| anyhow!("face threads: {e}"))?
        .commit_from_file(path)
        .map_err(|e| anyhow!("load face restorer {}: {e}", path.display()))?;
    for face in faces {
        let pad = (face.width.max(face.height) as f32 * 0.35) as i32;
        let cx = face.x as i32 + face.width as i32 / 2;
        let cy = face.y as i32 + face.height as i32 / 2;
        let side = (face.width.max(face.height) as i32 + 2 * pad).max(32);
        let mut crop = RgbaImage::new(side as u32, side as u32);
        let (sin, cos) = face.angle.sin_cos();
        for y in 0..side {
            for x in 0..side {
                let dx = x as f32 - side as f32 / 2.0;
                let dy = y as f32 - side as f32 / 2.0;
                let sx = (cx as f32 + cos * dx - sin * dy)
                    .round()
                    .clamp(0.0, image.width() as f32 - 1.0) as u32;
                let sy = (cy as f32 + sin * dx + cos * dy)
                    .round()
                    .clamp(0.0, image.height() as f32 - 1.0) as u32;
                crop.put_pixel(x as u32, y as u32, *image.get_pixel(sx, sy));
            }
        }
        let crop = image::imageops::resize(&crop, 512, 512, image::imageops::FilterType::Lanczos3);
        let mut data = vec![0f32; 3 * 512 * 512];
        for y in 0..512 {
            for x in 0..512 {
                let p = crop.get_pixel(x, y);
                let i = (y * 512 + x) as usize;
                for c in 0..3 {
                    data[c * 512 * 512 + i] = p[c] as f32 / 127.5 - 1.0;
                }
            }
        }
        let tensor = Tensor::from_array(Array4::from_shape_vec((1, 3, 512, 512), data)?)?;
        let out = s
            .run(ort::inputs![tensor])
            .map_err(|e| anyhow!("face restoration inference: {e}"))?;
        let value = &out[0];
        let arr = value.try_extract_array::<f32>()?;
        let shape = arr.shape();
        if shape.len() != 4 || shape[0] != 1 || shape[1] != 3 || shape[2] != 512 || shape[3] != 512
        {
            bail!("face restorer expected [1,3,512,512], got {shape:?}");
        }
        let mut restored = RgbaImage::new(512, 512);
        for y in 0..512 {
            for x in 0..512 {
                restored.put_pixel(
                    x,
                    y,
                    Rgba([
                        to_u8(arr[[0, 0, y as usize, x as usize]]),
                        to_u8(arr[[0, 1, y as usize, x as usize]]),
                        to_u8(arr[[0, 2, y as usize, x as usize]]),
                        255,
                    ]),
                );
            }
        }
        paste_aligned(image, &restored, cx, cy, side, face.angle, strength);
    }
    Ok(())
}
fn to_u8(x: f32) -> u8 {
    ((x.clamp(-1.0, 1.0) + 1.0) * 127.5).round() as u8
}
fn paste_aligned(
    base: &mut RgbaImage,
    restored: &RgbaImage,
    cx: i32,
    cy: i32,
    side: i32,
    angle: f32,
    strength: f32,
) {
    let (sin, cos) = angle.sin_cos();
    let half = side as f32 / 2.0;
    let r = (half * 1.42).ceil() as i32;
    let (w, h) = base.dimensions();
    for y in (cy - r).max(0)..(cy + r).min(h as i32) {
        for x in (cx - r).max(0)..(cx + r).min(w as i32) {
            let dx = x as f32 - cx as f32;
            let dy = y as f32 - cy as f32;
            let u = (cos * dx + sin * dy + half) / side as f32;
            let v = (-sin * dx + cos * dy + half) / side as f32;
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let edge = ((0.5 - (u - 0.5).abs().max((v - 0.5).abs())) / 0.08).clamp(0.0, 1.0);
            let mask = strength * edge * edge * (3.0 - 2.0 * edge);
            let src = restored.get_pixel(
                (u * restored.width() as f32) as u32,
                (v * restored.height() as f32) as u32,
            );
            let dst = *base.get_pixel(x as u32, y as u32);
            base.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    blend(dst[0], src[0], mask),
                    blend(dst[1], src[1], mask),
                    blend(dst[2], src[2], mask),
                    dst[3],
                ]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paste_face_clips_coords_and_fades_edges() {
        let mut base = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 255]));
        let face = RgbaImage::from_pixel(4, 4, Rgba([200, 100, 50, 255]));
        paste_face(
            &mut base,
            &face,
            FaceBox {
                x: 6,
                y: 6,
                width: 4,
                height: 4,
                angle: 0.0,
            },
            1.0,
        );
        assert_eq!(base.get_pixel(7, 7).0, [200, 100, 50, 255]);
        assert_eq!(base.get_pixel(6, 6).0, [0, 0, 0, 255]);
        paste_face(
            &mut base,
            &face,
            FaceBox {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
                angle: 0.0,
            },
            0.0,
        );
        assert_eq!(base.get_pixel(1, 1).0, [0, 0, 0, 255]);
    }
}
