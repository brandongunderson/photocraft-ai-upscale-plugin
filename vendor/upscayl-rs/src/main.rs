use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use upscayl::batch::BatchProcessor;
use upscayl::cli::{Cli, Commands, ModelsAction};
use upscayl::config::{ImageFormat, UpscaylOptions};
use upscayl::engine::{native::NativeNcnn, UpscaylEngine};
use upscayl::models::ModelRegistry;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Upscale {
            input,
            output,
            model,
            scale,
            custom_width,
            custom_height,
            tile_size,
            gpu_id,
            format,
            compression,
            tta,
            double,
            models_path,
            force_software,
            json,
        } => {
            if !input.exists() {
                eprintln!("Error: input file does not exist: {}", input.display());
                return ExitCode::from(2);
            }

            let img_format = ImageFormat::from_ext(&format);
            let out_path = output.unwrap_or_else(|| {
                let stem = input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("image");
                let s_label = scale.unwrap_or_else(|| ModelRegistry::get_scale(&model));
                let filename = format!(
                    "{}_upscayl_{}_{}x.{}",
                    stem,
                    model,
                    s_label,
                    img_format.extension()
                );
                input
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(filename)
            });

            let options = UpscaylOptions {
                model: model.clone(),
                scale,
                custom_width,
                custom_height,
                tile_size: Some(tile_size),
                gpu_id,
                format: img_format,
                compression,
                tta_mode: tta,
                double_upscale: double,
                models_path,
                force_software,
            };

            let pb = if !json {
                let bar = ProgressBar::new(100);
                bar.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% - {msg}")
                        .unwrap()
                        .progress_chars("#>-"),
                );
                bar.set_message("Upscaling image...");
                Some(bar)
            } else {
                None
            };

            let pb_clone = pb.clone();
            let cb = move |pct: f32| {
                if let Some(ref b) = pb_clone {
                    b.set_position(pct.clamp(0.0, 100.0) as u64);
                }
            };

            let result = match UpscaylEngine::upscale_file(&input, &out_path, &options, Some(&cb)) {
                Ok(r) => r,
                Err(e) => {
                    if let Some(b) = pb {
                        b.finish_and_clear();
                    }
                    eprintln!("Upscaling error: {}", e);
                    return ExitCode::from(1);
                }
            };

            if let Some(b) = pb {
                b.finish_with_message("Done!");
            }

            if json {
                if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                    println!("{}", json_str);
                }
            } else {
                println!("\n✨ Upscale Complete!");
                println!("Input:       {}", result.input_path);
                println!("Output:      {}", result.output_path);
                println!(
                    "Dimensions:  {}x{} ➔ {}x{} ({}x scale)",
                    result.original_width,
                    result.original_height,
                    result.upscaled_width,
                    result.upscaled_height,
                    result.scale
                );
                println!("Model:       {}", result.model);
                println!("Engine:      {}", result.engine);
                println!("Time:        {:.2}s", result.duration_ms as f64 / 1000.0);
            }

            ExitCode::SUCCESS
        }

        Commands::Batch {
            input_dir,
            output_dir,
            model,
            scale,
            format,
            double,
            force_software,
            json,
        } => {
            if !input_dir.is_dir() {
                eprintln!("Error: input directory not found: {}", input_dir.display());
                return ExitCode::from(2);
            }

            let img_format = ImageFormat::from_ext(&format);
            let options = UpscaylOptions {
                model,
                scale,
                format: img_format,
                double_upscale: double,
                force_software,
                ..Default::default()
            };

            let pb = if !json {
                let bar = ProgressBar::new(0);
                bar.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} images ({msg})")
                        .unwrap()
                        .progress_chars("#>-"),
                );
                Some(bar)
            } else {
                None
            };

            let pb_clone = pb.clone();
            let cb = move |current: usize, total: usize| {
                if let Some(ref b) = pb_clone {
                    b.set_length(total as u64);
                    b.set_position(current as u64);
                    b.set_message("Processing batch...");
                }
            };

            let summary = match BatchProcessor::process_directory(
                &input_dir,
                &output_dir,
                &options,
                Some(&cb),
            ) {
                Ok(s) => s,
                Err(e) => {
                    if let Some(b) = pb {
                        b.finish_and_clear();
                    }
                    eprintln!("Batch processing error: {}", e);
                    return ExitCode::from(1);
                }
            };

            if let Some(b) = pb {
                b.finish_with_message("Batch complete!");
            }

            if json {
                if let Ok(json_str) = serde_json::to_string_pretty(&summary) {
                    println!("{}", json_str);
                }
            } else {
                println!("\n📦 Batch Processing Summary:");
                println!("Total Images:       {}", summary.total_images);
                println!("Succeeded:          {}", summary.succeeded);
                println!("Failed:             {}", summary.failed);
                println!(
                    "Total Duration:     {:.2}s",
                    summary.total_duration_ms as f64 / 1000.0
                );
                println!(
                    "Avg Time per Image: {:.2}s",
                    summary.avg_time_per_image_ms / 1000.0
                );
                println!("Output Directory:   {}", output_dir.display());
            }

            if summary.failed > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }

        Commands::Models { action } => {
            match action.unwrap_or(ModelsAction::List) {
                ModelsAction::List => {
                    println!("Available Upscayl AI Models:\n");
                    for m in ModelRegistry::all() {
                        println!(" • {:<22} [{}x] - {}", m.id, m.scale, m.name);
                        println!("   {}", m.description);
                        println!("   Recommended for: {}\n", m.recommended_for);
                    }
                }
                ModelsAction::Info { model_id } => {
                    if let Some(m) = ModelRegistry::find(&model_id) {
                        println!("Model: {}", m.name);
                        println!("ID:    {}", m.id);
                        println!("Scale: {}x", m.scale);
                        println!("About: {}", m.description);
                        println!("Best:  {}", m.recommended_for);
                        println!("Param: {}", m.param_file);
                        println!("Bin:   {}", m.bin_file);
                    } else {
                        eprintln!("Model not found: {}", model_id);
                        return ExitCode::from(1);
                    }
                }
            }
            ExitCode::SUCCESS
        }

        Commands::Doctor => {
            println!("🔍 Upscayl Environment Diagnostics\n");

            let os = std::env::consts::OS;
            let arch = std::env::consts::ARCH;
            let num_cpus = rayon::current_num_threads();
            println!("Platform:       {} ({})", os, arch);
            println!("CPU Threads:    {}", num_cpus);

            print!("Native NCNN:    ");
            if let Some(bin) = NativeNcnn::find_binary() {
                println!("Found at {}", bin.display());
            } else {
                println!("Not found (Pure-Rust high-precision Lanczos3 engine will be used)");
            }

            let config = upscayl::config::UpscaylConfig::default();
            println!("Models Dir:     {}", config.models_dir.display());
            println!("Default Model:  {}", config.default_model);

            println!("\nModel Registry Check:");
            for m in ModelRegistry::all() {
                let status = if ModelRegistry::locate_files(&config.models_dir, &m.id).is_some() {
                    "✓ Local files present"
                } else {
                    "○ Software / Remote fallback"
                };
                println!(" - {:<22} {}", m.id, status);
            }

            ExitCode::SUCCESS
        }

        Commands::Benchmark { iterations, size } => {
            println!("⚡ Running Upscayl Super-Resolution Benchmark");
            println!("Image Dimensions: {}x{} RGBA", size, size);
            println!("Iterations:       {}\n", iterations);

            // Generate synthetic gradient test image
            let mut img_buf = image::RgbaImage::new(size, size);
            for y in 0..size {
                for x in 0..size {
                    let r = ((x as f32 / size as f32) * 255.0) as u8;
                    let g = ((y as f32 / size as f32) * 255.0) as u8;
                    let b = 180;
                    img_buf.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                }
            }
            let dyn_img = image::DynamicImage::ImageRgba8(img_buf);

            let start = Instant::now();
            for i in 0..iterations {
                let _ = upscayl::engine::software::upscale_software(&dyn_img, 4, None, None);
                print!("\rProgress: {}/{} iterations completed", i + 1, iterations);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            println!();

            let elapsed = start.elapsed();
            let per_iter = elapsed.as_secs_f64() / iterations as f64;
            let total_megapixels =
                (size as f64 * size as f64 * 16.0 * iterations as f64) / 1_000_000.0;
            let mp_per_sec = total_megapixels / elapsed.as_secs_f64();

            println!("\nBenchmark Results:");
            println!("Total Time:       {:.2?}", elapsed);
            println!("Per Image (4x):   {:.2} ms", per_iter * 1000.0);
            println!(
                "Throughput:       {:.1} Megapixels upscaled / second",
                mp_per_sec
            );

            ExitCode::SUCCESS
        }
    }
}
