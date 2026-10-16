use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Webp => "webp",
        }
    }

    pub fn from_ext(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::Webp,
            _ => ImageFormat::Png,
        }
    }
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpscaylOptions {
    pub model: String,
    pub scale: Option<u32>,
    pub custom_width: Option<u32>,
    pub custom_height: Option<u32>,
    pub tile_size: Option<u32>,
    pub gpu_id: Option<i32>,
    pub format: ImageFormat,
    pub compression: Option<u32>,
    pub tta_mode: bool,
    pub double_upscale: bool,
    pub models_path: Option<PathBuf>,
    pub force_software: bool,
}

impl Default for UpscaylOptions {
    fn default() -> Self {
        Self {
            model: "upscayl-standard-4x".to_string(),
            scale: Some(4),
            custom_width: None,
            custom_height: None,
            tile_size: Some(0), // 0 = auto
            gpu_id: None,
            format: ImageFormat::Png,
            compression: None,
            tta_mode: false,
            double_upscale: false,
            models_path: None,
            force_software: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpscaylConfig {
    pub default_model: String,
    pub default_scale: u32,
    pub default_format: ImageFormat,
    pub models_dir: PathBuf,
    pub binary_path: Option<PathBuf>,
}

impl Default for UpscaylConfig {
    fn default() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let default_models_dir = PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("upscayl")
            .join("models");

        Self {
            default_model: "upscayl-standard-4x".to_string(),
            default_scale: 4,
            default_format: ImageFormat::Png,
            models_dir: default_models_dir,
            binary_path: None,
        }
    }
}
