//! WVG CLI - Wireless Vector Graphics converter
//!
//! A command-line tool for parsing WVG files and converting them to SVG format.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use wvg::{BitStream, Converter, SvgConverter, WvgParser};

/// Verbosity level for logging output.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Verbosity {
    /// Only log success or failure messages.
    #[default]
    Quiet,
    /// Log header information and basic progress.
    Normal,
    /// Log all parsing details including element data.
    Verbose,
}

impl Verbosity {
    /// Returns the tracing filter string for this verbosity level.
    fn as_filter(&self) -> &'static str {
        match self {
            Verbosity::Quiet => "wvg=warn",
            Verbosity::Normal => "wvg=info",
            Verbosity::Verbose => "wvg=trace",
        }
    }
}

/// WVG to SVG converter
#[derive(Parser, Debug)]
#[command(name = "wvg")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input WVG file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output SVG file path
    #[arg(short, long)]
    output: PathBuf,

    /// Verbosity level
    #[arg(short, long, value_enum, default_value_t = Verbosity::default())]
    verbosity: Verbosity,
}

fn main() -> ExitCode {
    let args = Args::parse();

    // Initialize tracing with the appropriate filter level
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(args.verbosity.as_filter())),
        )
        .with_target(false)
        .with_level(true)
        .init();

    if let Err(e) = run(&args) {
        error!("Conversion failed: {}", e);
        return ExitCode::FAILURE;
    }

    info!("Conversion successful!");
    println!(
        "Successfully converted {} to {}",
        args.input.display(),
        args.output.display()
    );

    ExitCode::SUCCESS
}

/// Main conversion logic.
fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    // Read input file
    info!("Reading input file: {}", args.input.display());
    let data = fs::read(&args.input)?;
    info!("Read {} bytes", data.len());

    // Parse WVG
    info!("Parsing WVG data...");
    let mut bs = BitStream::new(&data);
    let parser = WvgParser::new(&mut bs);
    let document = parser.parse()?;
    info!(
        "Parsed {} elements",
        document.elements.len()
    );

    // Convert to SVG
    info!("Converting to SVG...");
    let converter = SvgConverter::new();
    let svg = converter.convert(&document)?;

    // Write output file
    info!("Writing output file: {}", args.output.display());
    fs::write(&args.output, svg)?;

    Ok(())
}
