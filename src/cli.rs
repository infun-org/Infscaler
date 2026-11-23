use std::io::{Write, stderr};
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
infscaler --input photo.jpg --output same.png --model realesrgan-x4plus --keep-size --tile 128"
)]
pub struct Cli {
    /// Input photo. If omitted, open the GUI.
    #[arg(long)]
    pub input: Option<std::path::PathBuf>,

    /// Output image (.png, .jpg, or .webp).
    #[arg(long)]
    pub output: Option<std::path::PathBuf>,

    /// Model name, matching a models/<name>.param + .bin pair.
    #[arg(long)]
    pub model: Option<String>,

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

    /// `auto`, or a GPU id (`0`). `cpu` is rejected by the shipped binaries.
    #[arg(long, default_value = "auto")]
    pub gpu: String,
}

pub fn run(cli: &Cli) -> Result<()> {
    let input = cli
        .input
        .clone()
        .ok_or_else(|| anyhow!("--input is required in CLI mode"))?;
    let output = cli
        .output
        .clone()
        .ok_or_else(|| anyhow!("--output is required in CLI mode"))?;
    let model = cli
        .model
        .clone()
        .ok_or_else(|| anyhow!("--model is required in CLI mode"))?;
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
        gpu,
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
    Ok(())
}
