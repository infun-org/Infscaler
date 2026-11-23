mod cli;
mod engine;
mod gui;
mod models;
mod paths;

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
    if cli.input.is_some() {
        cli::run(&cli)
    } else {
        gui::run().map_err(|err| anyhow!("{err}"))
    }
}
