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
    /// Input photo. If omitted, open the GUI.
    #[arg(long)]
    pub input: Option<std::path::PathBuf>,

    /// Output image (.png, .jpg, or .webp).
    #[arg(long)]
    pub output: Option<std::path::PathBuf>,

    /// Model name or `auto` (faces: DAT, otherwise Nomos2 OTF).
    #[arg(long, default_value = "auto")]
    pub model: Option<String>,

    /// In Auto mode, choose the fast Nomos8kSC GPU model.
    #[arg(long)]
    pub fast: bool,

    /// In Auto mode, use the faster Nomos2 OTF model for detected portraits.
    #[arg(long)]
    pub fast_ish: bool,

    /// Face restorer model (default RestoreFormerPlusPlus; ONNX file in models/).
    #[arg(long, num_args = 0..=1, default_missing_value = "RestoreFormerPlusPlus", default_value = "RestoreFormerPlusPlus")]
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
        output_scale: cli.scale,
        tile: cli.tile,
        tta: cli.tta,
        keep_size: cli.keep_size,
        denoise: cli.denoise,
        overlap: cli.overlap,
        gpu,
        face_restore: !cli.no_face_restore,
        face_strength: cli.face_strength,
        face_model: cli
            .face_restore
            .clone()
            .unwrap_or_else(|| "RestoreFormerPlusPlus".into()),
        fast: cli.fast,
        fast_ish: cli.fast_ish,
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
