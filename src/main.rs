#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::too_many_arguments)]

pub mod action;
pub mod components;
pub mod data;
pub mod runner;
pub mod tui;
pub mod utils;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::Result;

use crate::{
    runner::Runner,
    utils::{initialize_logging, initialize_panic_handler, version},
};

/// E2020 Data Viewer
#[derive(Parser, Debug)]
#[command(version=version(), about)]
struct Args {
    /// The input file to use
    #[arg(short, long)]
    file: PathBuf,
    /// Tick rate (ticks per second)
    #[arg(long, default_value_t = 4.0)]
    tick_rate: f64,
    /// Frame rate (frames per second)
    #[arg(long, default_value_t = 4.0)]
    frame_rate: f64,
    /// The dataset to read on load (optional)
    #[arg(short, long)]
    dataset: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logging()?;
    initialize_panic_handler()?;
    log::debug!("Starting in main");
    let args = Args::parse();
    let (tick_rate, frame_rate, file) = (
        args.tick_rate,
        args.frame_rate,
        args.file.as_os_str().to_string_lossy().to_string(),
    );
    log::debug!("Reading file: {file}");
    let mut app = Runner::new(tick_rate, frame_rate, file, args.dataset)?;
    app.run().await?;
    Ok(())
}
