//! OCR Module
//!
//! Handles text extraction from images using the tesseract-rs crate.

use anyhow::{Context, Result};
use image::{ImageBuffer, Rgb};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};
use rayon::prelude::*;
use std::path::PathBuf;
// Use the correct API and types from the provided source
use tesseract_rs::{TessPageIteratorLevel, TesseractAPI};

/// Represents a single recognized word with its metadata.
#[derive(Debug)]
pub struct OcrWord {
    pub text: String,
    /// Bounding box as a tuple: (x1, y1, x2, y2)
    pub bbox: (i32, i32, i32, i32),
    pub confidence: f32,
}

/// Holds all the recognized words from a single frame.
#[derive(Debug)]
pub struct OcrFrameResult {
    pub frame_index: usize,
    pub words: Vec<OcrWord>,
}

/// Gets the default location where this version of `tesseract-rs` caches its data.
/// The build script downloads language files here.
fn get_tessdata_dir() -> Result<PathBuf> {
    let base_path = if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").context("HOME env var not set")?;
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
    } else if cfg!(target_os = "linux") {
        let home = std::env::var("HOME").context("HOME env var not set")?;
        PathBuf::from(home).join(".tesseract-rs")
    } else if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").context("APPDATA env var not set")?;
        PathBuf::from(appdata)
    } else {
        panic!("Unsupported operating system");
    };
    Ok(base_path.join("tesseract-rs").join("tessdata"))
}

/// Performs OCR in parallel on a vector of image frames, extracting detailed word data.
pub fn perform_ocr_on_frames(
    frames: &[ImageBuffer<Rgb<u8>, Vec<u8>>],
    lang: &str,
) -> Result<Vec<OcrFrameResult>> {
    info!("Starting detailed OCR on {} frames using language '{}'...", frames.len(), lang);

    // Initialize one master API instance. It will be cloned for each thread.
    let api = TesseractAPI::new();
    let tessdata_dir = get_tessdata_dir().context("Could not determine tessdata directory")?;
    api.init(tessdata_dir.to_str().unwrap(), lang)
        .context(format!("Failed to initialize Tesseract with language '{}'", lang))?;

    let pb = ProgressBar::new(frames.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Running OCR [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );

    let results: Vec<OcrFrameResult> = frames
        .par_iter()
        .enumerate()
        .filter_map(|(index, frame)| {
            pb.inc(1);
            // Clone the master API for this thread. This is the intended usage.
            let api_clone = api.clone();

            if let Err(e) = api_clone.set_image(
                frame.as_raw(),
                frame.width() as i32,
                frame.height() as i32,
                3, // bytes per pixel for RGB
                (frame.width() * 3) as i32, // bytes per line
            ) {
                // Now, `e` contains the specific error variant, which will be printed.
                warn!("Tesseract failed to set image for frame {}: {}. Skipping.", index, e);
                return None;
            }

            if api_clone.recognize().is_err() {
                 warn!("Tesseract failed to recognize text for frame {}. Skipping.", index);
                 return None;
            }

            let iter = match api_clone.get_iterator() {
                Ok(iter) => iter,
                Err(_) => {
                    warn!("Failed to get result iterator for frame {}. Skipping.", index);
                    return None;
                }
            };
            
            let mut words = Vec::new();
            // Loop while the iterator can advance to the next word
            while iter.next(TessPageIteratorLevel::RIL_WORD).unwrap_or(false) {
                let word_text = match iter.get_utf8_text(TessPageIteratorLevel::RIL_WORD) {
                    Ok(text) => text.trim().to_string(),
                    Err(_) => continue,
                };

                if !word_text.is_empty() {
                    if let (Ok(bbox), Ok(confidence)) = (
                        iter.get_bounding_box(TessPageIteratorLevel::RIL_WORD),
                        iter.confidence(TessPageIteratorLevel::RIL_WORD)
                    ) {
                        words.push(OcrWord {
                            text: word_text,
                            bbox,
                            confidence,
                        });
                    }
                }
            }

            Some(OcrFrameResult {
                frame_index: index,
                words,
            })
        })
        .collect();

    pb.finish_with_message("OCR complete");
    info!("Successfully performed detailed OCR on {} frames.", results.len());

    Ok(results)
}