//! VideoDocParser - Core Library
//!
//! This file contains the primary logic for the application, orchestrating
//! the different modules to perform video processing, analysis, OCR, and
//! document generation.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

// Define modules for different functionalities
pub mod video_processor;
pub mod frame_analyzer;
pub mod ocr;
pub mod document_builder; // Make the new module public

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
    let pb = match video_processor::get_frame_count(&config.input_file) {
        Ok(count) if count > 0 => {
            let bar = ProgressBar::new(count);
            bar.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} Analyzing frames [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) [{elapsed_precise}<{eta}]")
                    .unwrap()
                    .progress_chars("##-"),
            );
            bar
        }
        _ => {
            warn!("Could not determine total frame count. Using spinner as fallback.");
            let bar = ProgressBar::new_spinner();
            bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} Analyzing frames... [{elapsed_precise}] {pos} frames processed")
                    .unwrap(),
            );
            bar
        }
    };
    pb.enable_steady_tick(std::time::Duration::from_millis(100));


    let frame_handler = |frame| {
        analyzer.process_frame(frame)?;
        pb.inc(1);
        Ok(())
    };
    video_processor::process_frames_stream(&config.input_file, frame_handler)?;
    let final_pos = pb.position();
    pb.finish_with_message(format!("Analyzed {} frames", final_pos));

    // 4. Finalize Analysis
    let analysis_result = analyzer.finish()?;
    if analysis_result.kept_frames.is_empty() {
        warn!("No unique frames were found based on the sensitivity settings. Exiting.");
        return Ok(());
    }
    let unique_frames = analysis_result.kept_frames;
    info!("Found {} unique frames to process.", unique_frames.len());

    // 5. Perform OCR on Unique Frames
    let ocr_results = ocr::perform_ocr_on_frames(&unique_frames, &config)
        .context("OCR processing failed")?;

    // 6. Build Document or Save Images
    info!("Generating output in '{}' format.", config.output_format);
    match config.output_format.as_str() {
        "pdf" => {
            info!("Building searchable PDF document...");
            let pdf_path = result_dir.join("document.pdf");
            document_builder::build_pdf(&unique_frames, &ocr_results, &pdf_path)
                .context("Failed to build PDF document")?;
            info!("Successfully created PDF: {:?}", pdf_path);
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
        _ => unreachable!(),
    }

    Ok(())
}