mod batch;
mod cli;
mod engine;
mod face;
mod gui;
mod i18n;
mod models;
mod ort_backend;
mod paths;
mod theme;
mod tile;

use anyhow::anyhow;
use clap::Parser;

fn main() {
    if let Err(err) = dispatch() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn dispatch() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    if let Some(spec) = cli.screenshot_spec() {
        gui::run_screenshot(spec).map_err(|err| anyhow!("{err}"))
    } else if cli.input.is_some() || cli.input_dir.is_some() || cli.list_models {
        cli::run(&cli)
    } else {
        gui::run().map_err(|err| anyhow!("{err}"))
    }
}
