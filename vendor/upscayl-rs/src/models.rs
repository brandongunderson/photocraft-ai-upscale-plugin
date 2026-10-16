use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub scale: u32,
    pub description: String,
    pub recommended_for: String,
    pub param_file: String,
    pub bin_file: String,
}

pub struct ModelRegistry;

impl ModelRegistry {
    pub fn all() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "upscayl-standard-4x".to_string(),
                name: "Upscayl Standard (Real-ESRGAN)".to_string(),
                scale: 4,
                description: "Balanced, general-purpose photo upscaler for high quality real-world photographs.".to_string(),
                recommended_for: "Portraits, real-world photos, landscapes, architectural shots".to_string(),
                param_file: "upscayl-standard-4x.param".to_string(),
                bin_file: "upscayl-standard-4x.bin".to_string(),
            },
            ModelInfo {
                id: "upscayl-lite-4x".to_string(),
                name: "Upscayl Lite".to_string(),
                scale: 4,
                description: "High-speed lightweight photo upscaler designed for fast batch processing and lower VRAM usage.".to_string(),
                recommended_for: "Quick previews, batch photo upscaling, mobile / low-power hardware".to_string(),
                param_file: "upscayl-lite-4x.param".to_string(),
                bin_file: "upscayl-lite-4x.bin".to_string(),
            },
            ModelInfo {
                id: "high-fidelity-4x".to_string(),
                name: "High Fidelity (HFA2k)".to_string(),
                scale: 4,
                description: "Upscayl's High Fidelity alias of Philip Hofmann's 4xHFA2k RRDBNet model, trained on anime images.".to_string(),
                recommended_for: "Try on illustrations and photographs; compare the preview for the specific image".to_string(),
                param_file: "high-fidelity-4x.param".to_string(),
                bin_file: "high-fidelity-4x.bin".to_string(),
            },
            ModelInfo {
                id: "remacri-4x".to_string(),
                name: "Remacri".to_string(),
                scale: 4,
                description: "Sharp and crisp restoration designed to reconstruct intricate cloth, brick, and hair details.".to_string(),
                recommended_for: "Macro photography, game textures, intricate patterns, fine textiles".to_string(),
                param_file: "remacri-4x.param".to_string(),
                bin_file: "remacri-4x.bin".to_string(),
            },
            ModelInfo {
                id: "ultramix-balanced-4x".to_string(),
                name: "UltraMix Balanced".to_string(),
                scale: 4,
                description: "Hybrid blend of UltraSharp and artistic enhancements, ideal for digital art and mixed media.".to_string(),
                recommended_for: "Midjourney/Stable Diffusion generation, digital art, mixed 3D renders".to_string(),
                param_file: "ultramix-balanced-4x.param".to_string(),
                bin_file: "ultramix-balanced-4x.bin".to_string(),
            },
            ModelInfo {
                id: "ultrasharp-4x".to_string(),
                name: "UltraSharp".to_string(),
                scale: 4,
                description: "Hyper-crisp, high-definition upscaler that extracts razor-sharp outlines and specular details.".to_string(),
                recommended_for: "3D renders, CGI, high-contrast imagery, game screenshots".to_string(),
                param_file: "ultrasharp-4x.param".to_string(),
                bin_file: "ultrasharp-4x.bin".to_string(),
            },
            ModelInfo {
                id: "digital-art-4x".to_string(),
                name: "Digital Art (Anime / Cartoon)".to_string(),
                scale: 4,
                description: "Specialized model for anime, 2D illustrations, cartoons, and vector-style graphics, producing clean line art and solid color fills.".to_string(),
                recommended_for: "Anime, manga, comics, digital sketches, icons, UI assets".to_string(),
                param_file: "digital-art-4x.param".to_string(),
                bin_file: "digital-art-4x.bin".to_string(),
            },
        ]
    }

    pub fn find(id: &str) -> Option<ModelInfo> {
        Self::all()
            .into_iter()
            .find(|m| m.id.eq_ignore_ascii_case(id))
    }

    pub fn get_scale(id: &str) -> u32 {
        Self::find(id).map(|m| m.scale).unwrap_or(4)
    }

    pub fn locate_files(models_dir: &Path, model_id: &str) -> Option<(PathBuf, PathBuf)> {
        let param = models_dir.join(format!("{}.param", model_id));
        let bin = models_dir.join(format!("{}.bin", model_id));
        if param.exists() && bin.exists() {
            Some((param, bin))
        } else {
            None
        }
    }
}

pub type Model = ModelInfo;
