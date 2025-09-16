//! VideoDocParser - Core Library
//!
//! This file contains the primary logic for the application, orchestrating
//! the different modules to perform video processing, analysis, OCR, and
//! document generation.

use anyhow::{Context, Result};
use log::{info, warn};
use rayon::prelude::*; // Import Rayon's parallel iterator traits
use std::fs;
use std::path::PathBuf;

// Define modules for different functionalities
pub mod video_processor;
pub mod frame_analyzer;
// pub mod ocr;
// pub mod element_detector;
// pub mod document_builder;

/// Application configuration structure.
#[derive(Debug)]
pub struct Config {
    pub input_file: PathBuf,
    pub output_dir: PathBuf,
    pub output_format: String,
    pub sensitivity: f64,
    pub lang: String,
    pub generate_index: bool,
}

/// The main function that orchestrates the video parsing process.
pub fn run(config: Config) -> Result<()> {
    info!("Initializing processing with config: {:?}", config);

    // 1. Setup Output Directory
    if !config.output_dir.exists() {
        fs::create_dir_all(&config.output_dir)
            .context("Failed to create output directory")?;
    }
    let result_dir = config.output_dir.join("result");
    if result_dir.exists() {
        fs::remove_dir_all(&result_dir)
            .context("Failed to clear existing result directory")?;
    }
    fs::create_dir_all(&result_dir)
        .context("Failed to create result directory")?;


    // 2. Initialize Frame Analyzer
    let mut analyzer = frame_analyzer::FrameAnalyzer::new(config.sensitivity, &config.output_dir)?;

    // 3. Process Video Stream
    info!("Starting video processing stream for: {:?}", config.input_file);    
    let frame_handler = |frame| {
        // This closure is called for each frame by the video processor.
        analyzer.process_frame(frame)
    };
    video_processor::process_frames_stream(&config.input_file, frame_handler)?;

    // 4. Finalize Analysis
    let analysis_result = analyzer.finish()?;
    
    if analysis_result.kept_frames.is_empty() {
        warn!("No unique frames were found based on the sensitivity settings. Exiting.");
        return Ok(());
    }
    
    let unique_frames = analysis_result.kept_frames;
    info!("Found {} unique frames to process.", unique_frames.len());

    // 5. Build Document or Save Images
    info!("Generating output in '{}' format.", config.output_format);
    match config.output_format.as_str() {
        "pdf" => {
            // TODO: document_builder::build_pdf(/*...args...*/)?,
            info!("PDF generation is not yet implemented.");
        }
        "md" => {
            // TODO: document_builder::build_markdown(/*...args...*/)?,
            info!("Markdown generation is not yet implemented.");
        }
        "img" => {
            info!("Saving unique frames as images to {:?}", result_dir);
            // Use a parallel iterator to save frames concurrently.
            unique_frames.par_iter().enumerate().try_for_each(|(i, frame)| -> Result<()> {
                let frame_path = result_dir.join(format!("frame_{:05}.png", i));
                frame.save(&frame_path)
                     .with_context(|| format!("Failed to save frame to {:?}", frame_path))?;
                Ok(())
            })?;

            info!("Successfully saved {} frames to {:?}", unique_frames.len(), result_dir);
        }
        _ => unreachable!(), // Should be caught by clap
    }

    Ok(())
}