use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use image::buffer::ConvertBuffer;
use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;

use crate::models::{self, cpu_backend_supports, ModelFiles, ModelInfo};
use crate::ort_backend;
use crate::paths;
use crate::tile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuChoice {
    Auto,
    Id(i32),
    Cpu,
}

impl GpuChoice {
    pub fn parse(text: &str) -> Result<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            other => {
                let id: i32 = other
                    .parse()
                    .map_err(|_| anyhow!("GPU must be auto, cpu, or an integer id (got {text})"))?;
                if id == -1 {
                    Ok(Self::Cpu)
                } else if id < 0 {
                    bail!("GPU id must be >= 0, or cpu");
                } else {
                    Ok(Self::Id(id))
                }
            }
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Auto => "auto".into(),
            Self::Id(id) => id.to_string(),
            Self::Cpu => "cpu".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuDevice {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct JobOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub model: String,
    pub output_scale: u32,
    pub tile: u32,
    pub tta: bool,
    pub keep_size: bool,
    pub denoise: bool,
    pub overlap: u32,
    pub gpu: GpuChoice,
    pub face_restore: bool,
    pub face_strength: f32,
    pub face_model: String,
    pub fast: bool,
    pub fast_ish: bool,
}

#[derive(Debug, Clone)]
pub struct RunOutcome {
    pub output: PathBuf,
    pub elapsed: Duration,
    pub input_size: (u32, u32),
    pub output_size: (u32, u32),
    pub backend: String,
    pub peak_rss_kb: u64,
}

#[derive(Debug)]
pub enum EngineEvent {
    Stage(String),
    Percent(f32),
    Log(String),
}

#[derive(Clone, Copy)]
enum BackendKind {
    Upscayl,
    RealEsrgan,
}

#[derive(Clone, Copy)]
enum OutFmt {
    Png,
    Jpeg,
    Webp,
}

struct TempPng(PathBuf);

impl Drop for TempPng {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn target_size(
    width: u32,
    height: u32,
    output_scale: u32,
    keep_size: bool,
) -> Result<(u32, u32)> {
    if width == 0 || height == 0 {
        bail!("Image has no pixels");
    }
    if keep_size {
        return Ok((width, height));
    }
    if !matches!(output_scale, 2 | 3 | 4) {
        bail!("Scale must be 2, 3, or 4 (use --keep-size to stay at the original size)");
    }
    let w = width
        .checked_mul(output_scale)
        .context("output width overflows")?;
    let h = height
        .checked_mul(output_scale)
        .context("output height overflows")?;
    Ok((w, h))
}

pub fn list_gpus() -> Vec<GpuDevice> {
    let Ok(output) = Command::new("timeout")
        .args(["20", "vulkaninfo", "--summary"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() && output.stdout.is_empty() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_vulkaninfo_summary(&text)
}

pub fn parse_vulkaninfo_summary(text: &str) -> Vec<GpuDevice> {
    let mut out = Vec::new();
    let mut id: Option<String> = None;
    let mut name: Option<String> = None;
    let mut kind: Option<String> = None;

    let flush = |out: &mut Vec<GpuDevice>,
                 id: &mut Option<String>,
                 name: &mut Option<String>,
                 kind: &mut Option<String>| {
        let Some(id) = id.take() else { return };
        let name = name.take().unwrap_or_else(|| "Vulkan device".into());
        let kind = kind.take().unwrap_or_default();
        let suffix = if kind.contains("INTEGRATED") {
            " (integrated)"
        } else if kind.contains("DISCRETE") {
            " (discrete)"
        } else if kind.contains("CPU") {
            " (cpu)"
        } else {
            ""
        };
        out.push(GpuDevice {
            label: format!("{id}: {name}{suffix}"),
            id,
        });
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("GPU") {
            if let Some(num) = rest.strip_suffix(':') {
                if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                    flush(&mut out, &mut id, &mut name, &mut kind);
                    id = Some(num.to_string());
                    continue;
                }
            }
        }
        if id.is_some() {
            if let Some((key, value)) = trimmed.split_once('=') {
                match key.trim() {
                    "deviceName" => name = Some(value.trim().to_string()),
                    "deviceType" => kind = Some(value.trim().to_string()),
                    _ => {}
                }
            }
        }
    }
    flush(&mut out, &mut id, &mut name, &mut kind);
    out
}

pub fn make_preview(path: &Path, max_edge: u32) -> Result<(Vec<u8>, u32, u32)> {
    let image = image::open(path)
        .with_context(|| format!("could not decode {}", path.display()))?
        .into_rgba8();
    let (w, h) = image.dimensions();
    let longest = w.max(h);
    if longest <= max_edge {
        return Ok((image.into_raw(), w, h));
    }
    let scale = max_edge as f32 / longest as f32;
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&image, nw, nh, image::imageops::FilterType::Triangle);
    let (nw, nh) = resized.dimensions();
    Ok((resized.into_raw(), nw, nh))
}

pub fn run_job(
    root: &Path,
    opts: &JobOptions,
    cancel: &AtomicBool,
    pid_slot: &AtomicU32,
    mut on_event: impl FnMut(EngineEvent),
) -> Result<RunOutcome> {
    if cancel.load(Ordering::SeqCst) {
        bail!("Cancelled");
    }
    if opts.tile != 0 && opts.tile < 32 {
        bail!("Tile size must be 0 (auto) or at least 32");
    }
    if !opts.input.is_file() {
        bail!("Input image not found: {}", opts.input.display());
    }
    if paths_equal(&opts.input, &opts.output) {
        bail!("Input and output are the same file. Choose a different output path.");
    }
    let fmt = output_format(&opts.output)?;
    if let Some(parent) = opts.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let (input_w, input_h) = image::image_dimensions(&opts.input)
        .with_context(|| format!("could not read {}", opts.input.display()))?;
    let (target_w, target_h) = target_size(input_w, input_h, opts.output_scale, opts.keep_size)?;

    let loaded = models::load_models(root);
    let requested_auto = opts.model.eq_ignore_ascii_case("auto");
    let has_detector = root.join("models/face_detection_yunet.onnx").is_file();
    if requested_auto && !has_detector {
        on_event(EngineEvent::Log("Auto face detection unavailable: models/face_detection_yunet.onnx is missing; treating image as no-face".into()));
    }
    let detected_input_faces = if (requested_auto || opts.face_restore) && has_detector {
        let face_started = Instant::now();
        let source = image::open(&opts.input)?.into_rgba8();
        let faces = crate::face::detect(&root.join("models/face_detection_yunet.onnx"), &source)?;
        on_event(EngineEvent::Log(format!(
            "Face detection: {} face(s)",
            faces.len()
        )));
        on_event(EngineEvent::Log(format!(
            "Face detection time: {:.2}s",
            face_started.elapsed().as_secs_f32()
        )));
        faces
    } else {
        Vec::new()
    };
    let selected_name = if requested_auto {
        let desired = if opts.fast {
            "4xNomos8kSC"
        } else if !detected_input_faces.is_empty() && !opts.fast_ish {
            "4xRealWebPhoto_v4_dat2"
        } else {
            "4xNomos2_otf_esrgan"
        };
        if loaded
            .iter()
            .any(|m| m.name == desired && !m.is_broken() && m.files_present())
        {
            desired
        } else if desired == "4xRealWebPhoto_v4_dat2"
            && loaded
                .iter()
                .any(|m| m.name == "4xNomos2_otf_esrgan" && m.files_present())
        {
            "4xNomos2_otf_esrgan"
        } else if desired == "4xNomos2_otf_esrgan"
            && loaded
                .iter()
                .any(|m| m.name == "4xNomos8kSC" && m.files_present())
        {
            "4xNomos8kSC"
        } else {
            bail!("Auto mode needs 4xNomos2_otf_esrgan, 4xRealWebPhoto_v4_dat2, or 4xNomos8kSC installed");
        }
    } else {
        opts.model.as_str()
    };
    if requested_auto {
        let choice = format!(
            "Auto chose {selected_name}{}",
            if opts.fast {
                " (Fast)"
            } else if opts.fast_ish {
                " (Fast-ish)"
            } else if detected_input_faces.is_empty() {
                " (no faces)"
            } else {
                " (faces detected)"
            }
        );
        on_event(EngineEvent::Stage(choice.clone()));
        on_event(EngineEvent::Log(choice));
    }
    let model = loaded
        .iter()
        .find(|m| m.name == selected_name)
        .ok_or_else(|| {
            anyhow!(
                "Model \"{}\" is not in models/ (need a .param + .bin pair, or a .onnx file)",
                selected_name
            )
        })?;
    if let Some(reason) = &model.broken_reason {
        bail!("Model \"{}\" is disabled as broken: {reason}", model.name);
    }
    if !model.files_present() {
        bail!(
            "Model files for {} are missing ({})",
            model.name,
            model.location()
        );
    }

    let peak = AtomicU64::new(0);
    ort_backend::sample_rss(&peak);
    let started = Instant::now();

    let mut work_input = opts.input.clone();
    let mut denoise_hold: Option<TempPng> = None;
    if opts.denoise {
        let Some(denoise) = models::denoise_model(&loaded) else {
            bail!("Denoise needs a SCUNet ONNX model in models/. Run scripts/export_models.sh.");
        };
        if denoise.name != model.name {
            on_event(EngineEvent::Stage(format!(
                "Denoising with {} ({})",
                denoise.name,
                denoise.kind().label()
            )));
            let tmp = TempPng(temp_png_path());
            run_one(
                root,
                denoise,
                &work_input,
                &tmp.0,
                opts,
                false,
                cancel,
                pid_slot,
                &peak,
                0.0,
                45.0,
                &mut on_event,
            )?;
            work_input = tmp.0.clone();
            denoise_hold = Some(tmp);
        }
    }
    let _keep_denoise = denoise_hold;

    let (from, to) = if opts.denoise {
        (45.0, 100.0)
    } else {
        (0.0, 100.0)
    };
    let temp = TempPng(temp_png_path());
    let backend = run_one(
        root,
        model,
        &work_input,
        &temp.0,
        opts,
        opts.tta,
        cancel,
        pid_slot,
        &peak,
        from,
        to,
        &mut on_event,
    )?;
    on_event(EngineEvent::Log(format!(
        "Enhancement time: {:.2}s",
        started.elapsed().as_secs_f32()
    )));

    let (native_w, native_h) =
        image::image_dimensions(&temp.0).context("could not read the enhanced image")?;
    on_event(EngineEvent::Percent(100.0));

    if native_w == target_w && native_h == target_h && matches!(fmt, OutFmt::Png) {
        move_file(&temp.0, &opts.output)?;
    } else {
        let resize_started = Instant::now();
        if native_w != target_w || native_h != target_h {
            on_event(EngineEvent::Stage(format!(
                "Lanczos3 resize {native_w}×{native_h} → {target_w}×{target_h}"
            )));
        } else {
            on_event(EngineEvent::Stage("Saving output".into()));
        }
        let image = image::open(&temp.0)?.into_rgba8();
        let image = if native_w != target_w || native_h != target_h {
            image::imageops::resize(
                &image,
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            image
        };
        save_rgba(&image, &opts.output, fmt)?;
        on_event(EngineEvent::Log(format!(
            "Resize/save time: {:.2}s",
            resize_started.elapsed().as_secs_f32()
        )));
    }

    if opts.face_restore && opts.face_strength > 0.0 {
        let detector = root.join("models/face_detection_yunet.onnx");
        let restorer = root
            .join("models")
            .join(format!("{}.onnx", opts.face_model));
        if !has_detector || !restorer.is_file() {
            on_event(EngineEvent::Log("Face restore skipped: optional YuNet / RestoreFormer++ ONNX files are missing; run scripts/setup_face_models.sh".into()));
        } else {
            on_event(EngineEvent::Stage(
                "Detecting faces in enhanced image".into(),
            ));
            let face_started = Instant::now();
            let mut image = image::open(&opts.output)?.into_rgba8();
            let mut faces = crate::face::detect(&detector, &image)?;
            if faces.is_empty() && !detected_input_faces.is_empty() {
                // Small/soft faces can score better before upscaling. Reuse
                // their boxes in final-image coordinates when the second pass
                // misses them; crop and restore still run on enhanced pixels.
                let sx = image.width() as f32 / input_w as f32;
                let sy = image.height() as f32 / input_h as f32;
                faces = detected_input_faces
                    .iter()
                    .map(|f| crate::face::FaceBox {
                        x: (f.x as f32 * sx) as u32,
                        y: (f.y as f32 * sy) as u32,
                        width: (f.width as f32 * sx) as u32,
                        height: (f.height as f32 * sy) as u32,
                        angle: f.angle,
                    })
                    .collect();
                on_event(EngineEvent::Log(format!("Using {} upscaled source face box(es) after enhanced-image detector missed them",faces.len())));
            }
            on_event(EngineEvent::Log(format!(
                "Enhanced-image detection: {} face(s), {:.2}s",
                faces.len(),
                face_started.elapsed().as_secs_f32()
            )));
            if faces.is_empty() {
                on_event(EngineEvent::Log(
                    "Face restore skipped: no faces detected".into(),
                ));
            } else {
                on_event(EngineEvent::Stage(format!(
                    "Restoring {} face(s) with {}",
                    faces.len(),
                    opts.face_model
                )));
                crate::face::restore(&restorer, &mut image, &faces, opts.face_strength)?;
                save_rgba(&image, &opts.output, fmt)?;
                on_event(EngineEvent::Log(format!(
                    "Face restore/paste time: {:.2}s",
                    face_started.elapsed().as_secs_f32()
                )));
            }
        }
    }

    ort_backend::sample_rss(&peak);
    let (output_w, output_h) =
        image::image_dimensions(&opts.output).context("could not read the saved image")?;
    Ok(RunOutcome {
        output: opts.output.clone(),
        elapsed: started.elapsed(),
        input_size: (input_w, input_h),
        output_size: (output_w, output_h),
        backend,
        peak_rss_kb: peak.load(Ordering::Relaxed),
    })
}

fn run_one(
    root: &Path,
    model: &ModelInfo,
    input: &Path,
    output_png: &Path,
    opts: &JobOptions,
    tta: bool,
    cancel: &AtomicBool,
    pid_slot: &AtomicU32,
    peak: &AtomicU64,
    percent_from: f32,
    percent_to: f32,
    on_event: &mut impl FnMut(EngineEvent),
) -> Result<String> {
    let span = percent_to - percent_from;
    let map = |p: f32| percent_from + span * (p.clamp(0.0, 100.0) / 100.0);
    match &model.files {
        ModelFiles::Onnx { path } => {
            if tta {
                bail!("TTA is only implemented for NCNN models. ONNX Runtime runs one pass.");
            }
            let tile = if opts.tile == 0 {
                tile::auto_tile(&model.name)
            } else {
                opts.tile
            };
            let overlap = opts.overlap.max(tile::min_overlap(&model.name));
            if overlap != opts.overlap {
                on_event(EngineEvent::Log(format!(
                    "{} keeps at least {overlap} px of overlap (asked for {})",
                    model.name, opts.overlap
                )));
            }
            let threads = ort_backend::thread_count();
            on_event(EngineEvent::Stage(format!(
                "{} with {} (ONNX Runtime CPU, {threads} threads, tile {tile}, overlap {overlap})",
                if model.denoise {
                    "Denoising"
                } else {
                    "Enhancing"
                },
                model.name,
            )));
            let image = image::open(input)
                .with_context(|| format!("could not decode {}", input.display()))?
                .into_rgba8();
            let enhanced = ort_backend::enhance(path, &image, tile, overlap, cancel, peak, |p| {
                on_event(EngineEvent::Percent(map(p)));
            })?;
            enhanced
                .save(output_png)
                .with_context(|| format!("save {}", output_png.display()))?;
            Ok(format!("onnxruntime-cpu ({threads} threads)"))
        }
        ModelFiles::Ncnn { .. } => {
            let (bin, kind) = select_backend(root, opts.gpu, &model.name)?;
            let models = paths::models_dir(root);
            let param_path = models.join(format!("{}.param", model.name));
            if param_path.as_os_str().len() >= 250 {
                bail!(
                    "Model path is too long for the upscaler (256-byte buffer): {}",
                    param_path.display()
                );
            }
            if !models.to_string_lossy().contains("models") {
                bail!(
                    "The model folder path must contain \"models\" or upscayl-bin refuses to start"
                );
            }
            let args = build_args(
                kind,
                input,
                output_png,
                &models,
                &model.name,
                model.scale,
                opts,
                tta,
            )?;
            on_event(EngineEvent::Stage(format!(
                "Upscaling with {} ({})",
                model.name,
                bin.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("backend")
            )));
            on_event(EngineEvent::Log(format!(
                "$ {} {}",
                bin.display(),
                args.iter()
                    .map(|a| if a.contains(' ') {
                        format!("\"{a}\"")
                    } else {
                        a.clone()
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            )));
            let tail =
                spawn_and_stream(
                    &bin,
                    &args,
                    root,
                    cancel,
                    pid_slot,
                    peak,
                    &mut |event| match event {
                        EngineEvent::Percent(p) => on_event(EngineEvent::Percent(map(p))),
                        other => on_event(other),
                    },
                )?;
            if !output_png.is_file() || std::fs::metadata(output_png)?.len() < 32 {
                bail!("Upscaler produced no output image.\n{tail}");
            }
            if tail_failed(&tail) {
                bail!("Upscaler reported an error.\n{tail}");
            }
            Ok(bin.display().to_string())
        }
    }
}

fn select_backend(root: &Path, gpu: GpuChoice, model: &str) -> Result<(PathBuf, BackendKind)> {
    let upscayl = paths::bin_dir(root).join("upscayl-bin");
    let realesrgan = paths::bin_dir(root).join("realesrgan-ncnn-vulkan");
    match gpu {
        GpuChoice::Cpu => {
            // upscayl-bin treats gpuid < 0 as invalid. The 20220424
            // realesrgan-ncnn-vulkan build prints "invalid gpu device" for -g -1
            // as well (checked on this machine). There is no CPU backend to run.
            bail!(
                "CPU mode is not available with the installed binaries. upscayl-bin rejects GPU ids below 0, \
                 and realesrgan-ncnn-vulkan 20220424 returns \"invalid gpu device\" for -g -1. Use auto or GPU 0."
            );
        }
        GpuChoice::Auto | GpuChoice::Id(_) => {
            if upscayl.is_file() {
                Ok((upscayl, BackendKind::Upscayl))
            } else if realesrgan.is_file() {
                if !cpu_backend_supports(model) {
                    bail!(
                        "upscayl-bin is missing, and realesrgan-ncnn-vulkan cannot load \"{model}\". Run scripts/setup.sh."
                    );
                }
                Ok((realesrgan, BackendKind::RealEsrgan))
            } else {
                bail!("No upscaler binary in bin/. Run scripts/setup.sh.");
            }
        }
    }
}

fn build_args(
    kind: BackendKind,
    input: &Path,
    output: &Path,
    models: &Path,
    model: &str,
    model_scale: u32,
    opts: &JobOptions,
    tta: bool,
) -> Result<Vec<String>> {
    let mut args = vec![
        "-i".into(),
        input.display().to_string(),
        "-o".into(),
        output.display().to_string(),
        "-m".into(),
        models.display().to_string(),
        "-n".into(),
        model.to_string(),
        "-t".into(),
        opts.tile.to_string(),
        // One process thread keeps the shared 660M heap from doubling up.
        "-j".into(),
        "1:1:1".into(),
        "-f".into(),
        "png".into(),
    ];
    match kind {
        BackendKind::Upscayl => {
            args.push("-z".into());
            args.push(model_scale.to_string());
            match opts.gpu {
                GpuChoice::Id(id) => {
                    args.push("-g".into());
                    args.push(id.to_string());
                }
                GpuChoice::Auto => {}
                GpuChoice::Cpu => bail!("internal: CPU was routed to upscayl-bin"),
            }
        }
        BackendKind::RealEsrgan => {
            // This build's -s is the network scale, not a post-scale.
            args.push("-s".into());
            args.push(model_scale.to_string());
            match opts.gpu {
                GpuChoice::Cpu => {
                    args.push("-g".into());
                    args.push("-1".into());
                }
                GpuChoice::Id(id) => {
                    args.push("-g".into());
                    args.push(id.to_string());
                }
                GpuChoice::Auto => {}
            }
        }
    }
    if tta {
        args.push("-x".into());
    }
    Ok(args)
}

fn spawn_and_stream(
    bin: &Path,
    args: &[String],
    root: &Path,
    cancel: &AtomicBool,
    pid_slot: &AtomicU32,
    peak: &AtomicU64,
    on_event: &mut impl FnMut(EngineEvent),
) -> Result<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .current_dir(root)
        // %.2f in the upscaler follows LC_NUMERIC. Force a dot so the bar parses.
        .env("LC_NUMERIC", "C");
    prepend_bin_lib_path(&mut cmd, root);

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to start {}", bin.display()))?;
    pid_slot.store(child.id(), Ordering::SeqCst);
    let stderr = child
        .stderr
        .take()
        .context("upscaler stderr was not piped")?;
    let mut reader = BufReader::new(stderr);
    let mut tail = String::new();
    let mut line_buf = Vec::new();

    loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            pid_slot.store(0, Ordering::SeqCst);
            bail!("Cancelled");
        }
        line_buf.clear();
        let n = reader
            .read_until(b'\n', &mut line_buf)
            .context("reading upscaler log")?;
        if n == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&line_buf);
        for piece in text.split(['\n', '\r']) {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            push_tail(&mut tail, piece);
            if let Some(kb) = ort_backend::vm_hwm_kb(child.id()) {
                peak.fetch_max(kb, Ordering::Relaxed);
            }
            ort_backend::sample_rss(peak);
            if let Some(percent) = parse_percent(piece) {
                on_event(EngineEvent::Percent(percent));
            } else {
                on_event(EngineEvent::Log(piece.to_string()));
            }
        }
    }

    let status = child.wait().context("waiting for upscaler")?;
    pid_slot.store(0, Ordering::SeqCst);
    if cancel.load(Ordering::SeqCst) {
        bail!("Cancelled");
    }
    if !status.success() {
        bail!("Upscaler exited with {status}.\n{tail}");
    }
    Ok(tail)
}

fn prepend_bin_lib_path(cmd: &mut Command, root: &Path) {
    let bin = paths::bin_dir(root);
    let mut paths = vec![bin.display().to_string()];
    if let Ok(current) = std::env::var("LD_LIBRARY_PATH") {
        if !current.is_empty() {
            paths.push(current);
        }
    }
    cmd.env("LD_LIBRARY_PATH", paths.join(":"));
}

fn parse_percent(line: &str) -> Option<f32> {
    let number = line.trim().strip_suffix('%')?.trim();
    if number.is_empty() {
        return None;
    }
    // glibc printf uses a comma when LC_NUMERIC is not C.
    let normalized = number.replace(',', ".");
    if !normalized.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    normalized.parse::<f32>().ok().map(|v| v.clamp(0.0, 100.0))
}

fn tail_failed(tail: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "Couldn't read the image",
        "Couldn't write the image",
        "unknown model",
        "Invalid GPU",
        "invalid gpu",
        "VK_ERROR",
        "vkAllocateMemory",
        "failed to load",
        "find_blob_index",
    ];
    NEEDLES.iter().any(|n| tail.contains(n))
}

fn push_tail(tail: &mut String, line: &str) {
    tail.push_str(line);
    tail.push('\n');
    const MAX: usize = 12_000;
    if tail.len() > MAX {
        let mut cut = tail.len() - MAX;
        while cut < tail.len() && !tail.is_char_boundary(cut) {
            cut += 1;
        }
        *tail = tail[cut..].to_string();
    }
}

fn temp_png_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("infscaler-{}-{nanos}.png", std::process::id()))
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn output_format(path: &Path) -> Result<OutFmt> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => Ok(OutFmt::Png),
        Some("jpg") | Some("jpeg") => Ok(OutFmt::Jpeg),
        Some("webp") => Ok(OutFmt::Webp),
        _ => bail!("Output must end in .png, .jpg, .jpeg, or .webp"),
    }
}

fn move_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if std::fs::rename(src, dst).is_err() {
        std::fs::copy(src, dst)
            .with_context(|| format!("copy {} → {}", src.display(), dst.display()))?;
        let _ = std::fs::remove_file(src);
    }
    Ok(())
}

fn save_rgba(image: &image::RgbaImage, path: &Path, fmt: OutFmt) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    match fmt {
        OutFmt::Png => {
            image
                .save(path)
                .with_context(|| format!("save {}", path.display()))?;
        }
        OutFmt::Jpeg => {
            let rgb: image::RgbImage = image.convert();
            let mut file = std::fs::File::create(path)?;
            let mut encoder = JpegEncoder::new_with_quality(&mut file, 95);
            encoder
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    ExtendedColorType::Rgb8,
                )
                .with_context(|| format!("save {}", path.display()))?;
        }
        OutFmt::Webp => {
            image::DynamicImage::ImageRgba8(image.clone())
                .save(path)
                .with_context(|| format!("save {}", path.display()))?;
        }
    }
    Ok(())
}

pub fn is_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png") | Some("jpg") | Some("jpeg") | Some("webp")
    )
}

pub fn default_output_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("enhanced");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}_infscaler.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_lines() {
        assert_eq!(parse_percent("12.50%"), Some(12.5));
        assert_eq!(parse_percent("  100.00%"), Some(100.0));
        assert_eq!(parse_percent("0,00%"), Some(0.0));
        assert_eq!(parse_percent("37,50%"), Some(37.5));
        assert_eq!(parse_percent("Upscayled Successfully!"), None);
        assert_eq!(parse_percent("100% done"), None);
    }

    #[test]
    fn target_sizes() {
        assert_eq!(target_size(320, 200, 4, false).unwrap(), (1280, 800));
        assert_eq!(target_size(320, 200, 4, true).unwrap(), (320, 200));
        assert!(target_size(10, 10, 1, false).is_err());
    }

    #[test]
    fn gpu_parse() {
        assert_eq!(GpuChoice::parse("auto").unwrap(), GpuChoice::Auto);
        assert_eq!(GpuChoice::parse("CPU").unwrap(), GpuChoice::Cpu);
        assert_eq!(GpuChoice::parse("-1").unwrap(), GpuChoice::Cpu);
        assert_eq!(GpuChoice::parse("0").unwrap(), GpuChoice::Id(0));
        assert!(GpuChoice::parse("gpu").is_err());
    }

    #[test]
    fn vulkaninfo_parser() {
        let text = "\
Devices:
GPU0:
    deviceName         = AMD Radeon 660M (RADV REMBRANDT)
    deviceType         = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU
GPU1:
    deviceType         = PHYSICAL_DEVICE_TYPE_CPU
    deviceName         = llvmpipe
";
        let gpus = parse_vulkaninfo_summary(text);
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0].id, "0");
        assert!(gpus[0].label.contains("660M"));
        assert!(gpus[0].label.contains("integrated"));
        assert!(gpus[1].label.contains("cpu"));
    }
}
