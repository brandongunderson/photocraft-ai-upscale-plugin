use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "upscayl")]
#[command(author = "Brandon Hubbard")]
#[command(version = "0.1.0")]
#[command(about = "High-performance native Rust port of Upscayl: AI image super-resolution, batch processing, and neural upscaling.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Upscale a single image file
    #[command(alias = "up")]
    Upscale {
        /// Path to input image file
        #[arg(short, long)]
        input: PathBuf,

        /// Path to save upscaled output image
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// AI Model to use (e.g. upscayl-standard-4x, ultrasharp-4x, digital-art-4x)
        #[arg(short, long, default_value = "upscayl-standard-4x")]
        model: String,

        /// Scale multiplier (e.g. 2, 3, 4, 8)
        #[arg(short, long)]
        scale: Option<u32>,

        /// Custom target output width in pixels (aspect ratio preserved if height omitted)
        #[arg(short = 'w', long)]
        custom_width: Option<u32>,

        /// Custom target output height in pixels
        #[arg(long)]
        custom_height: Option<u32>,

        /// Tile size for VRAM management (0 = auto, 256, 512, 1024)
        #[arg(short, long, default_value_t = 0)]
        tile_size: u32,

        /// GPU device ID (0 for primary GPU, or -1 for CPU)
        #[arg(short, long)]
        gpu_id: Option<i32>,

        /// Output image format
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Compression level (0-100 for JPEG/WebP)
        #[arg(short, long)]
        compression: Option<u32>,

        /// Enable TTA (Test-Time Augmentation) for higher quality 8x ensemble
        #[arg(short = 'x', long)]
        tta: bool,

        /// Perform double upscale pass (e.g. 4x -> 4x chaining)
        #[arg(long)]
        double: bool,

        /// Path to custom models directory
        #[arg(short = 'm', long)]
        models_path: Option<PathBuf>,

        /// Force pure-Rust Lanczos3 software engine (bypasses NCNN Vulkan)
        #[arg(long)]
        force_software: bool,

        /// Output result in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Batch upscale all images in a directory with parallel Rayon workers
    Batch {
        /// Directory containing input images
        #[arg(short, long)]
        input_dir: PathBuf,

        /// Directory to write upscaled images
        #[arg(short, long)]
        output_dir: PathBuf,

        /// AI Model to use
        #[arg(short, long, default_value = "upscayl-standard-4x")]
        model: String,

        /// Scale multiplier
        #[arg(short, long)]
        scale: Option<u32>,

        /// Output image format (png, jpg, webp)
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Double upscale pass
        #[arg(long)]
        double: bool,

        /// Force software mode
        #[arg(long)]
        force_software: bool,

        /// Output summary in JSON format
        #[arg(long)]
        json: bool,
    },

    /// List and inspect available Upscayl AI models
    Models {
        /// Subcommand for model management
        #[command(subcommand)]
        action: Option<ModelsAction>,
    },

    /// Diagnose system environment (GPU Vulkan, NCNN binaries, model files, CPU cores)
    Doctor,

    /// Benchmark image upscaling throughput on this machine
    Benchmark {
        /// Number of iterations
        #[arg(short, long, default_value_t = 10)]
        iterations: usize,

        /// Test image dimension (e.g. 256 for 256x256)
        #[arg(short, long, default_value_t = 256)]
        size: u32,
    },
}

#[derive(Subcommand, Debug)]
pub enum ModelsAction {
    /// List all built-in models
    List,
    /// View detailed information for a specific model
    Info {
        /// Model ID
        model_id: String,
    },
}
