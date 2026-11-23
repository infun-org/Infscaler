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

pub(crate) fn environment() -> Result<&'static ort::environment::Environment> {
    static CELL: OnceLock<ort::environment::Environment> = OnceLock::new();
    if let Some(env) = CELL.get() {
        return Ok(env);
    }
    let built = ort::init()
        .with_name("infscaler")
        .build()
        .map_err(|err| anyhow!("ONNX Runtime failed to start: {err}"))?;
    let _ = CELL.set(built);
    CELL.get()
        .context("ONNX Runtime environment was not stored")
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
    let asked = if tile == 0 { 128 } else { tile.max(8) };
    // DRCT, DAT, and SCUNet are traced at one square size. A different H/W
    // hits the constant-folded window mask and either crashes or, at the
    // trace size, disagrees with a full-frame forward.
    let tile = match fixed_square(&session) {
        Some(side) => {
            if asked != side {
                eprintln!("ONNX input is {side}×{side}; using that tile (asked for {asked})");
            }
            side
        }
        None => asked,
    };
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
            let cropped = crop_planar(
                &output.data,
                output.width as usize,
                output.height as usize,
                out_w,
                out_h,
            );
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
    let outputs = session.run(ort::inputs![tensor]).map_err(|err| {
        anyhow!("ONNX inference failed (try a smaller tile, a multiple of 8): {err}")
    })?;
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

fn extract_tile(
    image: &RgbaImage,
    origin_x: u32,
    origin_y: u32,
    tile_w: u32,
    tile_h: u32,
) -> (u32, u32, Vec<f32>) {
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

/// Static square spatial size, when the graph was exported without dynamic H/W.
fn fixed_square(session: &Session) -> Option<u32> {
    let input = session.inputs().first()?;
    let ort::value::ValueType::Tensor { shape, .. } = input.dtype() else {
        return None;
    };
    if shape.len() != 4 {
        return None;
    }
    let height = shape[2];
    let width = shape[3];
    if height <= 0 || width <= 0 || height != width {
        return None;
    }
    Some(height as u32)
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
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64};

    use image::{Rgba, RgbaImage};

    use super::{enhance, vm_hwm_kb};

    #[test]
    fn reads_own_peak_rss() {
        let kb = vm_hwm_kb(std::process::id()).expect("VmHWM line in /proc/self/status");
        assert!(kb > 0, "peak rss was {kb}");
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn synthetic(width: u32, height: u32) -> RgbaImage {
        let mut image = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = (x * 13 + y * 3) as u8;
                let g = (x * 5 + y * 17) as u8;
                let b = (x.wrapping_mul(9).wrapping_add(y * 11)) as u8;
                image.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
        image
    }

    fn max_diff(a: &RgbaImage, b: &RgbaImage) -> u8 {
        assert_eq!(a.dimensions(), b.dimensions());
        a.pixels()
            .zip(b.pixels())
            .flat_map(|(p, q)| (0..3).map(move |c| p[c].abs_diff(q[c])))
            .max()
            .unwrap_or(0)
    }

    /// One tile that covers the image, versus several overlapping tiles.
    /// Nearest-neighbor 4× is pointwise, so the blend must reproduce it.
    #[test]
    fn tiled_nearest_matches_one_tile() {
        let path = fixture("nearest_x4.onnx");
        let image = synthetic(20, 13);
        let cancel = AtomicBool::new(false);
        let peak = AtomicU64::new(0);
        let untiled = enhance(&path, &image, 64, 8, &cancel, &peak, |_| {}).expect("untiled");
        let tiled = enhance(&path, &image, 8, 4, &cancel, &peak, |_| {}).expect("tiled");
        assert_eq!(untiled.dimensions(), (80, 52));
        assert_eq!(tiled.dimensions(), untiled.dimensions());
        let diff = max_diff(&untiled, &tiled);
        assert!(diff <= 1, "tiled vs untiled max channel diff {diff}");
    }

    /// A 3×3 blur is wrong on the outer pixel of every tile (zero padding
    /// inside the graph). Overlap plus a ramp that hits 0 there has to hide it.
    #[test]
    fn tiled_blur_matches_one_tile() {
        let path = fixture("blur3.onnx");
        let image = synthetic(20, 13);
        let cancel = AtomicBool::new(false);
        let peak = AtomicU64::new(0);
        // Tile equals the width, so this is one tile: zero padding on the
        // left and right, reflect padding past the bottom, same as the
        // tiled path's edge tiles.
        let untiled = enhance(&path, &image, 20, 4, &cancel, &peak, |_| {}).expect("untiled");
        let tiled = enhance(&path, &image, 8, 4, &cancel, &peak, |_| {}).expect("tiled");
        assert_eq!(untiled.dimensions(), image.dimensions());
        let diff = max_diff(&untiled, &tiled);
        assert!(diff <= 1, "tiled blur max channel diff {diff}");
    }

    /// Asking for a tile other than the graph's static H/W must still run,
    /// at that fixed size, and match an explicit request for it.
    #[test]
    fn fixed_square_input_overrides_the_requested_tile() {
        let path = fixture("nearest_x4_fixed16.onnx");
        let image = synthetic(20, 13);
        let cancel = AtomicBool::new(false);
        let peak = AtomicU64::new(0);
        let forced = enhance(&path, &image, 8, 4, &cancel, &peak, |_| {}).expect("asked 8");
        let explicit = enhance(&path, &image, 16, 4, &cancel, &peak, |_| {}).expect("asked 16");
        assert_eq!(forced.dimensions(), (80, 52));
        let diff = max_diff(&forced, &explicit);
        assert!(diff <= 1, "fixed-size override max channel diff {diff}");
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
