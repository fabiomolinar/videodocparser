use image::{DynamicImage, ImageBuffer, Rgb};
use imagehash::{PerceptualHash, Hash};
use anyhow::{anyhow, Result};
use log::info;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

// Controls the precision of the perceptual hash. A larger size is more
// precise but slower.
const HASH_SIZE: (usize, usize) = (16, 16); // 256-bit hash

/// Holds the final results of the frame analysis.
pub struct AnalysisResult {
    pub kept_frames: Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    pub differences: Vec<u32>,
    pub removed_indices: Vec<usize>,
}

/// A stateful analyzer that processes frames one at a time to keep memory usage low.
pub struct FrameAnalyzer {
    sensitivity: f64,
    output_dir: PathBuf,
    start_time: Instant,
    frame_index: usize,
    hasher: PerceptualHash,
    max_distance: u32,
    last_hash: Option<Hash>,
    kept_frames: Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    differences: Vec<u32>,
    removed_indices: Vec<usize>,
}

impl FrameAnalyzer {
    /// Creates a new, initialized FrameAnalyzer.
    pub fn new(sensitivity: f64, output_dir: &Path) -> Result<Self> {
        let hasher = PerceptualHash::new()
            .with_image_size(HASH_SIZE.0, HASH_SIZE.1)
            .with_hash_size(HASH_SIZE.0, HASH_SIZE.1);
        let max_distance = (HASH_SIZE.0 * HASH_SIZE.1) as u32;

        Ok(FrameAnalyzer {
            sensitivity,
            output_dir: output_dir.to_path_buf(),
            start_time: Instant::now(),
            frame_index: 0,
            hasher,
            max_distance,
            last_hash: None,
            kept_frames: Vec::new(),
            differences: Vec::new(),
            removed_indices: Vec::new(),
        })
    }

    /// Processes a single frame, comparing it to the previous one.
    pub fn process_frame(&mut self, frame: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<()> {
        let dyn_img = DynamicImage::ImageRgb8(frame);
        let hash = self.hasher.hash(&dyn_img);

        if let Some(prev) = &self.last_hash {
            let dist = hamming_distance(prev, &hash)?;
            self.differences.push(dist);

            let diff_ratio = dist as f64 / self.max_distance as f64;

            if diff_ratio < (1.0 - self.sensitivity) {
                self.removed_indices.push(self.frame_index);
                self.frame_index += 1;
                return Ok(()); // Drop frame
            }
        }

        self.kept_frames.push(dyn_img.to_rgb8());
        self.last_hash = Some(hash);
        self.frame_index += 1;
        Ok(())
    }

    /// Finalizes the analysis, writes reports, and returns the results.
    pub fn finish(self) -> Result<AnalysisResult> {
        let elapsed = self.start_time.elapsed();

        // Save analysis log
        let stats_dir = self.output_dir.join("analysis");
        fs::create_dir_all(&stats_dir)?;
        let stats_path = stats_dir.join("frame_analysis.json");

        let report = serde_json::json!({
            "total_frames": self.frame_index,
            "removed": self.removed_indices.len(),
            "kept": self.kept_frames.len(),
            "removed_indices": self.removed_indices,
            "differences": self.differences,
        });

        fs::write(stats_path, serde_json::to_string_pretty(&report)?)?;

        info!(
            "Frame analysis complete in {:.2?}. Processed {}, Kept {}, removed {}.",
            elapsed,
            self.frame_index,
            self.kept_frames.len(),
            self.removed_indices.len()
        );

        Ok(AnalysisResult {
            kept_frames: self.kept_frames,
            differences: self.differences,
            removed_indices: self.removed_indices,
        })
    }
}


/// Calculates the Hamming distance between two perceptual hashes.
fn hamming_distance(a: &Hash, b: &Hash) -> Result<u32> {
    let a_bits = &a.bits;
    let b_bits = &b.bits;
    if a_bits.len() != b_bits.len() {
        return Err(anyhow!("Cannot compare hashes of different lengths."));
    }

    let distance = a_bits.iter()
        .zip(b_bits.iter())
        .filter(|(x, y)| x != y)
        .count() as u32;

    Ok(distance)
}