//! Tile grid and seam blending for ONNX inference.
//!
//! Weights ramp across the overlap and stay at 1 on an edge that already
//! touches the image border. The accumulator divides by the summed weights,
//! so a flat input stays flat and a seam is a weighted mix.

use anyhow::{bail, Result};

/// Where each tile starts along one axis.
///
/// The last tile keeps the same stride as the others, so it may extend past
/// `length`. The caller reflect-pads that overhang and crops it off. Pulling
/// the last tile flush with the image makes the overlap wider than the blend
/// ramp, and both tiles then contribute at full weight.
pub fn tile_origins(length: u32, tile: u32, overlap: u32) -> Vec<u32> {
    let tile = tile.max(1);
    if length == 0 || length <= tile {
        return vec![0];
    }
    let overlap = overlap.min(tile - 1);
    let stride = (tile - overlap).max(1);
    let mut origins = Vec::new();
    let mut pos = 0u32;
    loop {
        origins.push(pos);
        if pos + tile >= length {
            break;
        }
        let next = pos.saturating_add(stride);
        if next <= pos {
            break;
        }
        pos = next;
    }
    origins
}

/// Clamp overlap so the stride is at least 1 pixel.
pub fn clamp_overlap(tile: u32, overlap: u32) -> u32 {
    overlap.min(tile.saturating_sub(1))
}

/// Window-attention models disagree with a full-frame forward near a tile
/// edge. Keep at least this many input pixels of overlap so the blend stays
/// in the part of each tile that has enough context.
pub fn min_overlap(model_name: &str) -> u32 {
    let n = model_name.to_ascii_lowercase();
    if n.contains("drct") || n.contains("dat") || n.contains("scunet") {
        48
    } else if n.contains("span") {
        32
    } else {
        0
    }
}

/// Tile size used when the user leaves tile at 0 and the ONNX input is dynamic.
pub fn auto_tile(model_name: &str) -> u32 {
    let _ = model_name;
    128
}

/// Integer scale implied by one tile. Height and width must agree.
pub fn scale_from_shapes(in_h: u32, in_w: u32, out_h: u32, out_w: u32) -> Result<u32> {
    if in_h == 0 || in_w == 0 || out_h == 0 || out_w == 0 {
        bail!("empty tile shape {in_w}×{in_h} → {out_w}×{out_h}");
    }
    if !out_h.is_multiple_of(in_h) || !out_w.is_multiple_of(in_w) {
        bail!("model output {out_w}×{out_h} is not an integer scale of {in_w}×{in_h}");
    }
    let sh = out_h / in_h;
    let sw = out_w / in_w;
    if sh != sw || sh == 0 {
        bail!("model scale differs by axis ({sw}× width, {sh}× height)");
    }
    Ok(sh)
}

/// Per-pixel blend weights for one tile axis.
/// `overlap` is the fade length in the same pixels as `len` (output space).
pub fn axis_weights(len: usize, overlap: usize, touch_start: bool, touch_end: bool) -> Vec<f32> {
    let mut weights = vec![1.0f32; len];
    if overlap == 0 || len == 0 {
        return weights;
    }
    let fade = overlap.min(len);
    for i in 0..fade {
        // 0 on the outer pixel, 1 at the inner end of the ramp. A linear ramp
        // keeps a visible step where the two tiles disagree.
        let t = hann(i, fade);
        if !touch_start {
            weights[i] = weights[i].min(t);
        }
        if !touch_end {
            let idx = len - 1 - i;
            weights[idx] = weights[idx].min(t);
        }
    }
    weights
}

pub struct Accumulator {
    width: usize,
    height: usize,
    sum: Vec<f32>,
    weight: Vec<f32>,
}

impl Accumulator {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width.saturating_mul(height);
        Self {
            width,
            height,
            sum: vec![0.0; n * 3],
            weight: vec![0.0; n],
        }
    }

    /// `rgb` is planar NCHW-style RGB without the batch dim: `3 * tile_h * tile_w`.
    /// `weights` is `tile_h * tile_w`, row-major.
    pub fn add_tile(
        &mut self,
        origin_x: usize,
        origin_y: usize,
        tile_w: usize,
        tile_h: usize,
        rgb: &[f32],
        weights: &[f32],
    ) {
        let plane = tile_w.saturating_mul(tile_h);
        if rgb.len() < plane * 3 || weights.len() < plane || tile_w == 0 || tile_h == 0 {
            return;
        }
        for y in 0..tile_h {
            let dst_y = origin_y + y;
            if dst_y >= self.height {
                break;
            }
            for x in 0..tile_w {
                let dst_x = origin_x + x;
                if dst_x >= self.width {
                    break;
                }
                let src = y * tile_w + x;
                let w = weights[src];
                if w <= 0.0 {
                    continue;
                }
                let dst = dst_y * self.width + dst_x;
                self.weight[dst] += w;
                self.sum[dst * 3] += rgb[src] * w;
                self.sum[dst * 3 + 1] += rgb[plane + src] * w;
                self.sum[dst * 3 + 2] += rgb[plane * 2 + src] * w;
            }
        }
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.width * self.height * 4];
        for i in 0..self.width * self.height {
            let w = self.weight[i];
            let (r, g, b) = if w <= 1e-8 {
                (0.0, 0.0, 0.0)
            } else {
                (
                    self.sum[i * 3] / w,
                    self.sum[i * 3 + 1] / w,
                    self.sum[i * 3 + 2] / w,
                )
            };
            let px = &mut out[i * 4..i * 4 + 4];
            px[0] = to_u8(r);
            px[1] = to_u8(g);
            px[2] = to_u8(b);
            px[3] = 255;
        }
        out
    }
}

fn hann(index_from_edge: usize, fade: usize) -> f32 {
    if fade <= 1 {
        return 0.5;
    }
    let x = index_from_edge as f32 / (fade as f32 - 1.0);
    0.5 - 0.5 * (std::f32::consts::PI * x).cos()
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// 2D weights from independent axis ramps.
pub fn tile_weights(
    tile_w: usize,
    tile_h: usize,
    overlap_x: usize,
    overlap_y: usize,
    touch_left: bool,
    touch_right: bool,
    touch_top: bool,
    touch_bottom: bool,
) -> Vec<f32> {
    let xs = axis_weights(tile_w, overlap_x, touch_left, touch_right);
    let ys = axis_weights(tile_h, overlap_y, touch_top, touch_bottom);
    let mut out = vec![0.0; tile_w * tile_h];
    for y in 0..tile_h {
        for x in 0..tile_w {
            out[y * tile_w + x] = xs[x] * ys[y];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_cover_without_gaps() {
        let origins = tile_origins(10, 6, 2);
        assert_eq!(origins, vec![0, 4]);
        let origins = tile_origins(5, 8, 2);
        assert_eq!(origins, vec![0]);
        let origins = tile_origins(128, 128, 16);
        assert_eq!(origins, vec![0]);
        // 11 is not flush with stride 4. The last tile starts at 8 and
        // overhangs by 3, instead of being pulled back to 5 (overlap 5).
        let origins = tile_origins(11, 6, 2);
        assert_eq!(origins, vec![0, 4, 8]);
        for px in 0..11 {
            assert!(
                origins
                    .iter()
                    .any(|origin| px >= *origin && px < origin + 6),
                "pixel {px} uncovered"
            );
        }
    }

    #[test]
    fn hann_ramps_are_complementary() {
        let fade = 8;
        let left = axis_weights(16, fade, true, false);
        let right = axis_weights(16, fade, false, true);
        assert_eq!(left[15], 0.0);
        assert_eq!(right[0], 0.0);
        assert!((left[16 - fade] - 1.0).abs() < 1e-5);
        for i in 0..fade {
            let sum = left[16 - fade + i] + right[i];
            assert!((sum - 1.0).abs() < 1e-5, "i={i} sum={sum}");
        }
        assert_eq!(min_overlap("4xNomos2_hq_drct-l"), 48);
        assert_eq!(min_overlap("4xRealWebPhoto_v4_dat2"), 48);
        assert_eq!(min_overlap("scunet_color_real_psnr"), 48);
        assert_eq!(min_overlap("spanx4_ch48"), 32);
        assert_eq!(min_overlap("4xNomos2_otf_esrgan"), 0);
    }

    #[test]
    fn scale_rejects_mismatched_axes() {
        assert_eq!(scale_from_shapes(16, 16, 64, 64).unwrap(), 4);
        assert_eq!(scale_from_shapes(32, 40, 32, 40).unwrap(), 1);
        assert!(scale_from_shapes(16, 16, 64, 48).is_err());
        assert!(scale_from_shapes(16, 16, 30, 30).is_err());
    }

    #[test]
    fn border_weights_stay_positive() {
        let w = axis_weights(6, 2, true, false);
        assert_eq!(w[0], 1.0);
        assert!(w[5] < 1.0);
        assert!(w[5] >= 0.0);
        let interior = axis_weights(6, 2, false, false);
        assert!(interior[0] < 1.0 && interior[5] < 1.0);
        assert_eq!(interior[2], 1.0);
    }

    #[test]
    fn constant_tiles_stay_constant_across_a_seam() {
        // Two 4-wide tiles, overlap 2, image width 6. Value is 0.5 everywhere.
        let mut acc = Accumulator::new(6, 1);
        let rgb = vec![0.5; 3 * 4];
        let left = tile_weights(4, 1, 2, 0, true, false, true, true);
        let right = tile_weights(4, 1, 2, 0, false, true, true, true);
        acc.add_tile(0, 0, 4, 1, &rgb, &left);
        acc.add_tile(2, 0, 4, 1, &rgb, &right);
        let rgba = acc.to_rgba();
        for x in 0..6 {
            assert_eq!(rgba[x * 4], 128, "x={x} r");
            assert_eq!(rgba[x * 4 + 1], 128, "x={x} g");
            assert_eq!(rgba[x * 4 + 2], 128, "x={x} b");
        }
    }

    #[test]
    fn seam_blends_between_zero_and_one() {
        // Overlap 4 so the Hann ramp has pixels strictly between 0 and 1.
        // A 2 px ramp is only the endpoints (0 and 1).
        let mut acc = Accumulator::new(12, 1);
        let zeros = vec![0.0; 3 * 8];
        let ones = vec![1.0; 3 * 8];
        let left = tile_weights(8, 1, 4, 0, true, false, true, true);
        let right = tile_weights(8, 1, 4, 0, false, true, true, true);
        acc.add_tile(0, 0, 8, 1, &zeros, &left);
        acc.add_tile(4, 0, 8, 1, &ones, &right);
        let rgba = acc.to_rgba();
        assert_eq!(rgba[0], 0);
        assert_eq!(rgba[11 * 4], 255);
        let mid = rgba[6 * 4];
        assert!(mid > 0 && mid < 255, "overlap pixel was {mid}");
    }
}
