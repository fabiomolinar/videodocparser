//! VideoDocParser - Core Library
//!
//! This file contains the primary logic for the application, orchestrating
//! the different modules to perform video processing, analysis, OCR, and
//! document generation.

use crate::frame_analyzer::AnalysisResult;
use crate::ocr::OcrFrameResult;
use anyhow::{Context, Result};
use image::{ImageBuffer, Rgb};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

// Define modules for different functionalities
pub mod document_builder;
pub mod frame_analyzer;
pub mod ocr;
pub mod video_processor;

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

/// The main entry point that constructs and runs the processing pipeline.
pub fn run(config: Config) -> Result<()> {
    Pipeline::new(config)?.run()
}

/// Encapsulates the entire video processing pipeline.
struct Pipeline {
    config: Config,
    result_dir: PathBuf,
}

impl Pipeline {
    /// Creates a new pipeline and sets up its initial state.
    fn new(config: Config) -> Result<Self> {
        let result_dir = config.output_dir.join("result");
        Ok(Self { config, result_dir })
    }

    /// Executes all stages of the pipeline in sequence.
    fn run(&self) -> Result<()> {
        self.setup_directories().context("Failed to set up directories")?;

        let analysis_result = self
            .analyze_frames()
            .context("Frame analysis failed")?;

        if analysis_result.kept_frames.is_empty() {
            warn!("No unique frames were found based on the sensitivity settings. Exiting.");
            return Ok(());
        }

        info!(
            "Found {} unique frames to process.",
            analysis_result.kept_frames.len()
        );

        let ocr_results = self
            .perform_ocr(&analysis_result.kept_frames)
            .context("OCR processing failed")?;

        self.generate_output(&analysis_result.kept_frames, &ocr_results)
            .context("Failed to generate output")?;

        Ok(())
    }

    /// Creates or clears the necessary output directories.
    fn setup_directories(&self) -> Result<()> {
        if !self.config.output_dir.exists() {
            fs::create_dir_all(&self.config.output_dir)?
        }
        if self.result_dir.exists() {
            fs::remove_dir_all(&self.result_dir)?
        }
        fs::create_dir_all(&self.result_dir)?;
        Ok(())
    }

    /// Runs the streaming video analysis stage.
    fn analyze_frames(&self) -> Result<AnalysisResult> {
        let mut analyzer =
            frame_analyzer::FrameAnalyzer::new(self.config.sensitivity, &self.config.output_dir)?;

        let pb = match video_processor::get_frame_count(&self.config.input_file) {
            Ok(count) if count > 0 => {
                let bar = ProgressBar::new(count);
                bar.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} Analyzing frames [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) [{elapsed_precise}<{eta}]")?
                        .progress_chars("##-"),
                );
                bar
            }
            _ => {
                warn!("Could not determine total frame count. Using spinner as fallback.");
                let bar = ProgressBar::new_spinner();
                bar.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} Analyzing frames... [{elapsed_precise}] {pos} frames processed")?,
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

        video_processor::process_frames_stream(&self.config.input_file, frame_handler)?;

        let final_pos = pb.position();
        pb.finish_with_message(format!("Analyzed {} frames", final_pos));

        analyzer.finish()
    }

    /// Runs the parallel OCR stage.
    fn perform_ocr(&self, frames: &[ImageBuffer<Rgb<u8>, Vec<u8>>]) -> Result<Vec<OcrFrameResult>> {
        ocr::perform_ocr_on_frames(frames, &self.config)
    }

    /// Generates the final output file(s) based on the format specified in the config.
    fn generate_output(
        &self,
        frames: &[ImageBuffer<Rgb<u8>, Vec<u8>>],
        ocr_results: &[OcrFrameResult],
    ) -> Result<()> {
        info!("Generating output in '{}' format.", self.config.output_format);
        match self.config.output_format.as_str() {
            "pdf" => {
                info!("Building searchable PDF document...");
                let pdf_path = self.result_dir.join("document.pdf");
                document_builder::build_pdf(frames, ocr_results, &pdf_path)?;
                info!("Successfully created PDF: {:?}", pdf_path);
            }
            "md" => {
                info!("Markdown generation is not yet implemented.");
            }
            "img" => {
                info!("Saving unique frames as images to {:?}", self.result_dir);
                frames.par_iter().enumerate().try_for_each(|(i, frame)| -> Result<()> {
                    let frame_path = self.result_dir.join(format!("frame_{:05}.png", i));
                    frame.save(&frame_path)
                         .with_context(|| format!("Failed to save frame to {:?}", frame_path))?;
                    Ok(())
                })?;
                info!("Successfully saved {} frames to {:?}", frames.len(), self.result_dir);
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}