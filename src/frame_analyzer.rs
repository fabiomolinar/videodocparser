use img_hash::image::{DynamicImage, ImageBuffer, Rgb};
use img_hash::{HasherConfig, ImageHash};
use anyhow::Result;
use log::info;
use std::fs;
use std::path::Path;

pub struct AnalysisResult {
    pub kept_frames: Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    pub differences: Vec<u32>,
    pub removed_indices: Vec<usize>,
}

pub fn analyze_frames(
    frames: Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    sensitivity: f64,
    output_dir: &Path,
) -> Result<AnalysisResult> {
    let mut kept_frames = Vec::new();
    let mut differences = Vec::new();
    let mut removed_indices = Vec::new();

    // Decide upfront hash size (controls precision and speed)
    let hash_size = (8, 8); // 64-bit hash
    let hasher = HasherConfig::new()
        .hash_size(hash_size.0, hash_size.1)
        .to_hasher();
    let max_distance = (hash_size.0 * hash_size.1) as u32;
    let mut last_hash: Option<ImageHash> = None;

    for (i, frame) in frames.into_iter().enumerate() {
        // Convert ImageBuffer to DynamicImage so it works with img_hash
        let dyn_img = DynamicImage::ImageRgb8(frame.clone());
        let hash = hasher.hash_image(&dyn_img);

        if let Some(prev) = &last_hash {
            let dist = hash.dist(prev);
            differences.push(dist);

            // Normalize if you want sensitivity to be a percentage (0.0–1.0)
            let diff_ratio = dist as f64 / max_distance as f64;

            if diff_ratio < (1.0 - sensitivity) {
                removed_indices.push(i);
                continue; // drop frame
            }
        }

        kept_frames.push(frame);
        last_hash = Some(hash);
    }

    // Save analysis log
    let stats_dir = output_dir.join("analysis");
    fs::create_dir_all(&stats_dir)?;
    let stats_path = stats_dir.join("frame_analysis.json");

    let report = serde_json::json!({
        "total_frames": kept_frames.len() + removed_indices.len(),
        "removed": removed_indices.len(),
        "kept": kept_frames.len(),
        "removed_indices": removed_indices,
        "differences": differences,
    });

    fs::write(stats_path, serde_json::to_string_pretty(&report)?)?;

    info!(
        "Frame analysis complete. Kept {}, removed {}.",
        kept_frames.len(),
        removed_indices.len()
    );

    Ok(AnalysisResult {
        kept_frames,
        differences,
        removed_indices,
    })
}
