//! VideoDocParser - Core Library
//!
//! This file contains the primary logic for the application, orchestrating
//! the different modules to perform video processing, analysis, OCR, and
//! document generation.

use anyhow::{Context, Result};
use log::{info, warn};
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


    // 2. Process Video
    info!("Starting video processing for: {:?}", config.input_file);
    let frames = video_processor::extract_frames(&config.input_file)?;
    
    if frames.is_empty() {
        warn!("No frames were extracted from the video. Exiting.");
        return Ok(());
    }

    info!("Extracted {} frames. Analyzing for unique content...", frames.len());

    // 3. Frame Analysis (Placeholder)
    //let analysis = frame_analyzer::analyze_frames(frames, config.sensitivity, &config.output_dir)?;
    let unique_frames = frames;
    info!("Found {} unique frames to process.", unique_frames.len());


    // 4. OCR & Element Detection (Placeholder)
    // TODO: Loop through unique_frames and perform OCR and element detection.
    
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
            info!("Saving unique frames as images...");
            for (i, frame) in unique_frames.iter().enumerate() {
                let file_name = format!("frame_{:05}.png", i);
                let path = &result_dir.join(file_name);

                img_hash::image::DynamicImage::ImageRgb8(frame.clone())
                    .save(&path)
                    .with_context(|| format!("Failed to save frame to {:?}", path))?;

                info!("Saved {:?}", path);
            }
            info!("Successfully saved {} frames to {:?}", unique_frames.len(), result_dir);
        }
        _ => unreachable!(), // Should be caught by clap
    }

    Ok(())
}

