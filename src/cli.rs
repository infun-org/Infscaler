use std::io::{stderr, Write};
use std::sync::atomic::{AtomicBool, AtomicU32};

use anyhow::{anyhow, Context, Result};
use clap::Parser;

use crate::engine::{self, EngineEvent, GpuChoice, JobOptions};
use crate::paths;

#[derive(Debug, Parser)]
#[command(
    name = "infscaler",
    version,
    about = "Enhance and upscale photos locally with NCNN-Vulkan",
    after_help = "\
With no --input, InfScaler opens the window.\n\
\n\
Examples:\n  \
infscaler\n  \
infscaler --input photo.jpg --output out.png --model 4xNomos8kSC\n  \
infscaler --input photo.jpg --output same.png --model 4xNomos2_otf_esrgan --keep-size --denoise"
)]
pub struct Cli {
    /// Process supported images in this folder (top level only).
    #[arg(long)]
    pub input_dir: Option<std::path::PathBuf>,
    #[arg(long)]
    pub output_dir: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = 1)]
    pub jobs: usize,
    #[arg(long, default_value_t = 4)]
    pub threads: usize,
    #[arg(long, default_value = "sharpen")]
    pub mode: String,
    #[arg(long, default_value = "general")]
    pub preset: String,
    /// Input photo. If omitted, open the GUI.
    #[arg(long)]
    pub input: Option<std::path::PathBuf>,

    /// Output image (.png, .jpg, or .webp).
    #[arg(long)]
    pub output: Option<std::path::PathBuf>,

    /// Model name or `auto` (selection follows --preset).
    #[arg(long, default_value = "auto")]
    pub model: Option<String>,

    /// In Auto mode, choose the fast Nomos8kSC GPU model.
    #[arg(long)]
    pub fast: bool,

    /// Face restorer model (default RestoreFormerPlusPlus; ONNX file in models/).
    #[arg(long, num_args = 0..=1, default_missing_value = "RestoreFormerPlusPlus")]
    pub face_restore: Option<String>,

    /// Disable automatic face restoration.
    #[arg(long)]
    pub no_face_restore: bool,

    /// Face blend strength, from 0 to 1.
    #[arg(long, default_value_t = 0.8)]
    pub face_strength: f32,

    /// Run the 8-orientation TTA pass. Much slower.
    #[arg(long)]
    pub tta: bool,

    /// Tile size in pixels. 0 lets the backend pick from the GPU heap.
    #[arg(long, default_value_t = 0)]
    pub tile: u32,

    /// Enhance at the model scale, then Lanczos3 back to the original size.
    #[arg(long)]
    pub keep_size: bool,

    /// Output scale when not using --keep-size. 2, 3, or 4.
    #[arg(long, default_value_t = 4)]
    pub scale: u32,

    /// `auto`, a GPU id (`0`), or `cpu`. CPU runs ONNX models. NCNN binaries reject it.
    #[arg(long, default_value = "auto")]
    pub gpu: String,

    /// Run the SCUNet denoiser before the selected model.
    #[arg(long)]
    pub denoise: bool,

    /// ONNX tile overlap in input pixels. NCNN ignores this.
    #[arg(long, default_value_t = 16)]
    pub overlap: u32,

    /// Print installed models and exit.
    #[arg(long)]
    pub list_models: bool,
}

pub fn run(cli: &Cli) -> Result<()> {
    if let Some(input_dir) = &cli.input_dir {
        let mode = match cli.mode.as_str() {
            "sharpen" => crate::batch::OutputMode::Sharpen,
            "upscale2" => crate::batch::OutputMode::Upscale2,
            "upscale4" => crate::batch::OutputMode::Upscale4,
            other => {
                return Err(anyhow!(
                    "--mode must be sharpen, upscale2, or upscale4 (got {other})"
                ))
            }
        };
        if !(1..=3).contains(&cli.jobs) {
            return Err(anyhow!("--jobs must be 1, 2, or 3"));
        }
        if !matches!(cli.preset.as_str(), "people" | "general" | "fast") {
            return Err(anyhow!("--preset must be people, general, or fast"));
        }
        let root = paths::find_root().context("Could not find the InfScaler project (scripts/setup.sh). Set INFSCALER_HOME or run from the repo.")?;
        let control = std::sync::Arc::new(crate::batch::BatchControl::default());
        crate::batch::run(
            &root,
            &crate::batch::BatchOptions {
                input_dir: input_dir.clone(),
                input_files: None,
                output_dir: cli.output_dir.clone(),
                jobs: cli.jobs,
                threads: cli.threads,
                mode,
                preset: cli.preset.clone(),
            },
            control,
            |line| eprintln!("{line}"),
        )?;
        return Ok(());
    }
    if cli.list_models {
        return print_models();
    }
    let input = cli
        .input
        .clone()
        .ok_or_else(|| anyhow!("--input is required in CLI mode"))?;
    let output = cli
        .output
        .clone()
        .ok_or_else(|| anyhow!("--output is required in CLI mode"))?;
    let model = cli.model.clone().unwrap_or_else(|| "auto".into());
    if !matches!(cli.mode.as_str(), "sharpen" | "upscale2" | "upscale4") {
        return Err(anyhow!("--mode must be sharpen, upscale2, or upscale4"));
    }
    if !(0.0..=1.0).contains(&cli.face_strength) {
        return Err(anyhow!("--face-strength must be between 0 and 1"));
    }
    let root = paths::find_root().context(
        "Could not find the InfScaler project (scripts/setup.sh). Set INFSCALER_HOME or run from the repo.",
    )?;
    let gpu = GpuChoice::parse(&cli.gpu)?;
    let opts = JobOptions {
        input: input.clone(),
        output: output.clone(),
        model: model.clone(),
        output_scale: match cli.mode.as_str() {
            "upscale2" => 2,
            "upscale4" => 4,
            _ => cli.scale,
        },
        tile: cli.tile,
        tta: cli.tta,
        keep_size: cli.keep_size || cli.mode == "sharpen",
        denoise: cli.denoise,
        overlap: cli.overlap,
        gpu,
        face_restore: cli.face_restore.is_some() && !cli.no_face_restore,
        face_strength: cli.face_strength,
        face_model: cli
            .face_restore
            .clone()
            .unwrap_or_else(|| "RestoreFormerPlusPlus".into()),
        fast: cli.fast,
        preset: cli.preset.clone(),
    };

    let cancel = AtomicBool::new(false);
    let pid = AtomicU32::new(0);
    let outcome = engine::run_job(&root, &opts, &cancel, &pid, |event| match event {
        EngineEvent::Percent(p) => {
            eprint!("\r  {p:6.2}%");
            let _ = stderr().flush();
        }
        EngineEvent::Stage(text) | EngineEvent::Log(text) => {
            eprintln!("\r{text}");
        }
    })?;
    eprintln!();
    let secs = outcome.elapsed.as_secs_f64();
    println!("model:   {model}");
    println!(
        "input:   {} ({}×{})",
        input.display(),
        outcome.input_size.0,
        outcome.input_size.1
    );
    println!(
        "output:  {} ({}×{})",
        outcome.output.display(),
        outcome.output_size.0,
        outcome.output_size.1
    );
    println!("backend: {}", outcome.backend);
    println!("gpu:     {}", gpu.label());
    println!("tile:    {}", cli.tile);
    println!("tta:     {}", if cli.tta { "on" } else { "off" });
    println!(
        "keep:    {}",
        if cli.keep_size { "original size" } else { "no" }
    );
    println!("time:    {secs:.2}s");
    println!("rss_kb:  {}", outcome.peak_rss_kb);
    println!("denoise: {}", if cli.denoise { "on" } else { "off" });
    println!("overlap: {}", cli.overlap);
    Ok(())
}

fn print_models() -> Result<()> {
    let root = paths::find_root().context(
        "Could not find the InfScaler project (scripts/setup.sh). Set INFSCALER_HOME or run from the repo.",
    )?;
    let models = crate::models::load_models(&root);
    if models.is_empty() {
        println!("(no models)");
        return Ok(());
    }
    for model in models {
        let sha = if model.sha256.is_empty() {
            "-".to_string()
        } else {
            model.sha256.clone()
        };
        println!(
            "{}\t{}\t{}x\t{}\t{}{}",
            model.name,
            model.kind().label(),
            model.scale,
            if model.denoise { "denoise" } else { "enhance" },
            sha,
            model
                .broken_reason
                .as_deref()
                .map(|reason| format!("\tBROKEN: {reason}"))
                .unwrap_or_default()
        );
    }
    Ok(())
}
