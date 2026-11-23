//! Tiled ONNX Runtime inference on the CPU.
//!
//! Images are RGB float32 in NCHW, range 0–1. The scale is read from the
//! first tile's output shape, so a 4× network and a 1× denoiser share this path.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;

use anyhow::{anyhow, bail, Context, Result};
use image::RgbaImage;
use ndarray::Array4;
use ort::session::Session;
use ort::value::Tensor;

use crate::tile::{self, Accumulator};

pub fn thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn environment() -> Result<&'static ort::environment::Environment> {
    static CELL: OnceLock<ort::environment::Environment> = OnceLock::new();
    if let Some(env) = CELL.get() {
        return Ok(env);
    }
    let built = ort::init()
        .with_name("infscaler")
        .build()
        .map_err(|err| anyhow!("ONNX Runtime failed to start: {err}"))?;
    let _ = CELL.set(built);
    CELL.get().context("ONNX Runtime environment was not stored")
}

pub fn enhance(
    model_path: &Path,
    image: &RgbaImage,
    tile: u32,
    overlap: u32,
    cancel: &AtomicBool,
    peak: &AtomicU64,
    mut on_percent: impl FnMut(f32),
) -> Result<RgbaImage> {
    if cancel.load(Ordering::SeqCst) {
        bail!("Cancelled");
    }
    let threads = thread_count();
    let env = environment()?;
    let mut session = Session::builder(env)
        .map_err(|err| anyhow!("{err}"))?
        .with_intra_threads(threads)
        .map_err(|err| anyhow!("{err}"))?
        .commit_from_file(model_path)
        .map_err(|err| anyhow!("failed to load {}: {err}", model_path.display()))?;

    let (img_w, img_h) = image.dimensions();
    let tile = if tile == 0 { 128 } else { tile.max(8) };
    let overlap = tile::clamp_overlap(tile, overlap);
    let xs = tile::tile_origins(img_w, tile, overlap);
    let ys = tile::tile_origins(img_h, tile, overlap);
    let total = (xs.len() * ys.len()).max(1) as f32;

    let mut scale: Option<u32> = None;
    let mut canvas: Option<Accumulator> = None;
    let mut done = 0usize;

    for &y in &ys {
        for &x in &xs {
            if cancel.load(Ordering::SeqCst) {
                bail!("Cancelled");
            }
            sample_rss(peak);
            let (tile_w, tile_h, nchw) = extract_tile(image, x, y, tile, tile);
            let output = infer_tile(&mut session, &nchw, tile_h, tile_w)?;
            let found = tile::scale_from_shapes(tile_h, tile_w, output.height, output.width)?;
            if let Some(prev) = scale {
                if prev != found {
                    bail!("model scale changed between tiles ({prev} then {found})");
                }
            } else {
                scale = Some(found);
                canvas = Some(Accumulator::new(
                    (img_w as usize).saturating_mul(found as usize),
                    (img_h as usize).saturating_mul(found as usize),
                ));
            }
            let scale = found;
            let valid_w = (img_w - x).min(tile_w);
            let valid_h = (img_h - y).min(tile_h);
            let out_w = (valid_w * scale) as usize;
            let out_h = (valid_h * scale) as usize;
            let cropped = crop_planar(&output.data, output.width as usize, output.height as usize, out_w, out_h);
            let weights = tile::tile_weights(
                out_w,
                out_h,
                (overlap * scale) as usize,
                (overlap * scale) as usize,
                x == 0,
                x + valid_w >= img_w,
                y == 0,
                y + valid_h >= img_h,
            );
            canvas.as_mut().expect("canvas").add_tile(
                (x * scale) as usize,
                (y * scale) as usize,
                out_w,
                out_h,
                &cropped,
                &weights,
            );
            done += 1;
            on_percent((done as f32 / total) * 100.0);
        }
    }

    let canvas = canvas.context("model produced no tiles")?;
    let rgba = canvas.to_rgba();
    let scale = scale.unwrap_or(1);
    RgbaImage::from_raw(img_w * scale, img_h * scale, rgba)
        .context("blended image had the wrong number of bytes")
}

struct TileOut {
    width: u32,
    height: u32,
    data: Vec<f32>,
}

fn infer_tile(session: &mut Session, nchw: &[f32], height: u32, width: u32) -> Result<TileOut> {
    let array = Array4::from_shape_vec((1, 3, height as usize, width as usize), nchw.to_vec())
        .context("NCHW tile shape")?;
    let tensor = Tensor::from_array(array).map_err(|err| anyhow!("tensor: {err}"))?;
    let outputs = session
        .run(ort::inputs![tensor])
        .map_err(|err| anyhow!("ONNX inference failed (try a smaller tile, a multiple of 8): {err}"))?;
    if outputs.len() == 0 {
        bail!("model returned no outputs");
    }
    let value = &outputs[0];
    let extracted = value
        .try_extract_array::<f32>()
        .map_err(|err| anyhow!("model output was not a float tensor: {err}"))?;
    let shape = extracted.shape();
    if shape.len() != 4 || shape[0] != 1 || shape[1] != 3 {
        bail!("expected output shape [1, 3, H, W], got {shape:?}");
    }
    let out_h = shape[2] as u32;
    let out_w = shape[3] as u32;
    let mut data = Vec::with_capacity((3 * out_h * out_w) as usize);
    for c in 0..3 {
        for y in 0..out_h as usize {
            for x in 0..out_w as usize {
                data.push(extracted[[0, c, y, x]]);
            }
        }
    }
    Ok(TileOut {
        width: out_w,
        height: out_h,
        data,
    })
}

fn extract_tile(image: &RgbaImage, origin_x: u32, origin_y: u32, tile_w: u32, tile_h: u32) -> (u32, u32, Vec<f32>) {
    let (img_w, img_h) = image.dimensions();
    let mut data = vec![0.0f32; (3 * tile_h * tile_w) as usize];
    let plane = (tile_h * tile_w) as usize;
    for y in 0..tile_h {
        for x in 0..tile_w {
            let sx = reflect((origin_x + x) as i32, img_w as i32);
            let sy = reflect((origin_y + y) as i32, img_h as i32);
            let p = image.get_pixel(sx, sy);
            let i = (y * tile_w + x) as usize;
            data[i] = p[0] as f32 / 255.0;
            data[plane + i] = p[1] as f32 / 255.0;
            data[plane * 2 + i] = p[2] as f32 / 255.0;
        }
    }
    (tile_w, tile_h, data)
}

fn crop_planar(src: &[f32], src_w: usize, src_h: usize, dst_w: usize, dst_h: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; 3 * dst_w * dst_h];
    let src_plane = src_w * src_h;
    let dst_plane = dst_w * dst_h;
    for c in 0..3 {
        for y in 0..dst_h.min(src_h) {
            for x in 0..dst_w.min(src_w) {
                out[c * dst_plane + y * dst_w + x] = src[c * src_plane + y * src_w + x];
            }
        }
    }
    out
}

fn reflect(v: i32, len: i32) -> u32 {
    if len <= 1 {
        return 0;
    }
    let period = 2 * (len - 1);
    let mut m = v.rem_euclid(period);
    if m >= len {
        m = period - m;
    }
    m as u32
}

#[cfg(test)]
mod tests {
    use super::vm_hwm_kb;

    #[test]
    fn reads_own_peak_rss() {
        let kb = vm_hwm_kb(std::process::id()).expect("VmHWM line in /proc/self/status");
        assert!(kb > 0, "peak rss was {kb}");
    }
}

pub fn sample_rss(peak: &AtomicU64) {
    if let Some(kb) = vm_hwm_kb(std::process::id()) {
        peak.fetch_max(kb, Ordering::Relaxed);
    }
}

pub fn vm_hwm_kb(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("VmHWM:") else {
            continue;
        };
        let kb = rest.split_whitespace().next()?;
        return kb.parse().ok();
    }
    None
}
