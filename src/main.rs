//! VideoDocParser - Main Application Entrypoint
//!
//! This file is responsible for parsing command-line arguments, initializing
//! the application environment (like logging), and dispatching the core
//! processing logic.

use clap::Parser;
use log::{error, info};
use std::path::PathBuf;
use videodocparser::run;

/// A command-line tool that converts video recordings of documents into searchable digital formats.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the input video file (e.g., lecture.mp4)
    #[arg(short, long)]
    input: PathBuf,

    /// Directory to save the output files
    #[arg(short, long)]
    output: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Pdf)]
    format: OutputFormat,

    /// Frame-to-frame comparison sensitivity threshold (0.0 to 1.0)
    #[arg(short, long, default_value_t = 0.95)]
    sensitivity: f64,

    /// OCR language (e.g., "eng" for English, "spa" for Spanish)
    #[arg(short, long, default_value_t = String::from("eng"))]
    lang: String,

    /// Generate an optional JSON index file with metadata
    #[arg(long)]
    index: bool,

    /// Logging verbosity level
    #[arg(long, value_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum OutputFormat {
    Pdf,
    Md,
    Img,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum LogLevel {
    Error,
    Info,
    Debug,
}

fn main() {
    let args = Args::parse();

    // 1. Initialize Logger
    let log_level = match args.log_level {
        LogLevel::Error => "error",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("Starting VideoDocParser...");

    // 2. Validate input path
    if !args.input.exists() {
        error!("Input file does not exist: {:?}", args.input);
        std::process::exit(1);
    }
    
    // 3. Create a configuration object from arguments
    let config = videodocparser::Config {
        input_file: args.input,
        output_dir: args.output,
        output_format: match args.format {
            OutputFormat::Pdf => "pdf".to_string(),
            OutputFormat::Md => "md".to_string(),
            OutputFormat::Img => "img".to_string(),
        },
        sensitivity: args.sensitivity,
        lang: args.lang,
        generate_index: args.index,
    };

    // 4. Run the main application logic
    if let Err(e) = run(config) {
        error!("Application failed: {}", e);
        std::process::exit(2);
    }

    info!("Processing completed successfully.");
    std::process::exit(0);
}


