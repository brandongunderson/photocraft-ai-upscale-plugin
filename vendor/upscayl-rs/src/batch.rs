use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use walkdir::WalkDir;

use crate::config::UpscaylOptions;
use crate::engine::{UpscaleResult, UpscaylEngine};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_images: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_duration_ms: u64,
    pub avg_time_per_image_ms: f64,
    pub results: Vec<UpscaleResult>,
}

pub struct BatchProcessor;

impl BatchProcessor {
    pub fn is_image_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tiff" | "tif"
            )
        } else {
            false
        }
    }

    pub fn process_directory(
        input_dir: &Path,
        output_dir: &Path,
        options: &UpscaylOptions,
        progress_cb: Option<&(dyn Fn(usize, usize) + Sync)>,
    ) -> std::io::Result<BatchSummary> {
        let start = Instant::now();

        // 1. Collect all image paths
        let mut image_paths: Vec<PathBuf> = Vec::new();
        for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && Self::is_image_file(entry.path()) {
                image_paths.push(entry.path().to_path_buf());
            }
        }

        std::fs::create_dir_all(output_dir)?;

        let total = image_paths.len();
        let counter = AtomicUsize::new(0);
        let succeeded_counter = AtomicUsize::new(0);
        let failed_counter = AtomicUsize::new(0);

        // 2. Process in parallel with Rayon
        let results: Vec<UpscaleResult> = image_paths
            .par_iter()
            .filter_map(|input_path| {
                let file_stem = input_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("image");
                let out_ext = options.format.extension();
                let scale_label = options.scale.unwrap_or(4);
                let out_name = format!(
                    "{}_upscayl_{}_{}x.{}",
                    file_stem, options.model, scale_label, out_ext
                );
                let output_path = output_dir.join(out_name);

                let res = match UpscaylEngine::upscale_file(input_path, &output_path, options, None)
                {
                    Ok(r) => {
                        succeeded_counter.fetch_add(1, Ordering::Relaxed);
                        Some(r)
                    }
                    Err(e) => {
                        eprintln!("Failed to upscale {}: {}", input_path.display(), e);
                        failed_counter.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                };

                let current = counter.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(cb) = progress_cb {
                    cb(current, total);
                }

                res
            })
            .collect();

        let elapsed = start.elapsed().as_millis() as u64;
        let succeeded = succeeded_counter.load(Ordering::SeqCst);
        let failed = failed_counter.load(Ordering::SeqCst);
        let avg_time = if succeeded > 0 {
            elapsed as f64 / succeeded as f64
        } else {
            0.0
        };

        Ok(BatchSummary {
            total_images: total,
            succeeded,
            failed,
            total_duration_ms: elapsed,
            avg_time_per_image_ms: avg_time,
            results,
        })
    }
}
