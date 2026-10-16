pub mod native;
pub mod software;

use image::{DynamicImage, GenericImageView, ImageReader};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

use crate::config::UpscaylOptions;
use crate::models::ModelRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpscaleResult {
    pub input_path: String,
    pub output_path: String,
    pub original_width: u32,
    pub original_height: u32,
    pub upscaled_width: u32,
    pub upscaled_height: u32,
    pub model: String,
    pub scale: u32,
    pub duration_ms: u64,
    pub engine: String,
}

pub struct UpscaylEngine;

impl UpscaylEngine {
    pub fn upscale_file(
        input: &Path,
        output: &Path,
        options: &UpscaylOptions,
        progress_callback: Option<&dyn Fn(f32)>,
    ) -> std::io::Result<UpscaleResult> {
        let start = Instant::now();

        // 1. Read input image and dimensions
        let reader = ImageReader::open(input)?
            .with_guessed_format()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let img = reader
            .decode()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let (orig_w, orig_h) = img.dimensions();

        let model_scale = ModelRegistry::get_scale(&options.model);
        let target_scale = options.scale.unwrap_or(model_scale);

        let native_bin = if !options.force_software {
            native::NativeNcnn::find_binary()
        } else {
            None
        };

        let mut engine_used = "pure-rust-lanczos3".to_string();

        if let Some(bin_path) = native_bin {
            // Native NCNN Vulkan execution
            engine_used = "ncnn-vulkan".to_string();

            if options.double_upscale {
                // Pass 1: Intermediate 4x upscale to tempfile
                let temp_dir = tempfile::tempdir()?;
                let temp_pass1 = temp_dir.path().join("pass1.png");

                let mut pass1_options = options.clone();
                pass1_options.scale = Some(4);
                pass1_options.custom_width = None;
                pass1_options.custom_height = None;

                native::NativeNcnn::execute(
                    &bin_path,
                    input,
                    &temp_pass1,
                    &pass1_options,
                    progress_callback,
                )?;

                // Pass 2: Final upscale from pass 1
                native::NativeNcnn::execute(
                    &bin_path,
                    &temp_pass1,
                    output,
                    options,
                    progress_callback,
                )?;
            } else {
                native::NativeNcnn::execute(&bin_path, input, output, options, progress_callback)?;
            }
        } else {
            // Pure Rust high-precision image resampling engine
            if let Some(cb) = progress_callback {
                cb(10.0);
            }

            let processed = if options.double_upscale {
                // Pass 1: 4x upscale
                let pass1 = software::upscale_software(&img, 4, None, None);
                if let Some(cb) = progress_callback {
                    cb(50.0);
                }
                let pass1_dyn = DynamicImage::ImageRgba8(pass1);

                // Pass 2: 4x upscale of intermediate pass
                software::upscale_software(
                    &pass1_dyn,
                    4,
                    options.custom_width,
                    options.custom_height,
                )
            } else {
                software::upscale_software(
                    &img,
                    target_scale,
                    options.custom_width,
                    options.custom_height,
                )
            };

            if let Some(cb) = progress_callback {
                cb(90.0);
            }

            // Ensure parent directory exists
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Save in requested format
            let dyn_output = DynamicImage::ImageRgba8(processed);
            let is_jpeg = options.format == crate::config::ImageFormat::Jpeg
                || output
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| {
                        e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg")
                    });

            if is_jpeg {
                dyn_output
                    .to_rgb8()
                    .save(output)
                    .map_err(std::io::Error::other)?;
            } else {
                dyn_output.save(output).map_err(std::io::Error::other)?;
            }

            if let Some(cb) = progress_callback {
                cb(100.0);
            }
        }

        // Read final upscaled dimensions
        let final_reader = ImageReader::open(output)?
            .with_guessed_format()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let (up_w, up_h) = final_reader
            .into_dimensions()
            .unwrap_or((orig_w * target_scale, orig_h * target_scale));

        let duration = start.elapsed().as_millis() as u64;

        Ok(UpscaleResult {
            input_path: input.display().to_string(),
            output_path: output.display().to_string(),
            original_width: orig_w,
            original_height: orig_h,
            upscaled_width: up_w,
            upscaled_height: up_h,
            model: options.model.clone(),
            scale: target_scale,
            duration_ms: duration,
            engine: engine_used,
        })
    }
}
