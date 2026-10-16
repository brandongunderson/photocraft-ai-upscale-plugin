//! upscayl-rs: High-performance native Rust port and CLI/library for Upscayl.
//! AI image super-resolution, batch processing, and neural upscaling.

pub mod batch;
pub mod cli;
pub mod config;
pub mod engine;
pub mod models;
pub mod tiling;

pub use config::{ImageFormat, UpscaylConfig, UpscaylOptions};
pub use engine::{UpscaleResult, UpscaylEngine};
pub use models::{Model, ModelInfo, ModelRegistry};
