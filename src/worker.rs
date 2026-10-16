//! Headless worker for the PhotoCraft plugin host. The stdin lifetime is its cancellation
//! channel; losing the host cancels native NCNN processing and reaps its child.
use std::io::Read;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use photocraft_ai_upscale_plugin::{
    Result,
    engine::{self, Settings},
    error,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Job {
    api_version: u32,
    input: PathBuf,
    output: PathBuf,
    options: WorkerOptions,
    output_size: OutputSize,
    color_space: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputSize {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerOptions {
    engine: engine::EngineKind,
    model: String,
    scale: u32,
    detail: u32,
    tile_size: u32,
    tta: bool,
}

impl Job {
    fn settings(&self) -> Result<Settings> {
        if self.api_version != 1
            || self.color_space != "srgb"
            || !self.input.is_absolute()
            || !self.output.is_absolute()
            || self.input == self.output
        {
            return Err(error(
                "Unsupported worker protocol, color space or invalid image paths",
            ));
        }
        if !engine::MODELS
            .iter()
            .any(|(id, _)| *id == self.options.model)
        {
            return Err(error("Unknown AI model."));
        }
        let decoder = image::codecs::png::PngDecoder::new(std::io::BufReader::new(
            std::fs::File::open(&self.input)?,
        ))?;
        use image::ImageDecoder;
        if decoder.color_type() != image::ColorType::Rgba8 {
            return Err(error("The plugin requires an 8-bit RGBA sRGB PNG input."));
        }
        let (width, height) = decoder.dimensions();
        if width > 32768
            || height > 32768
            || u64::from(width) * u64::from(height) > engine::MAX_INPUT_PIXELS
        {
            return Err(error(
                "Input exceeds 32,768 pixels per side or 100 megapixels.",
            ));
        }
        let settings = Settings {
            engine: self.options.engine,
            model: self.options.model.clone(),
            scale: self.options.scale,
            detail: self.options.detail,
            tile_size: self.options.tile_size,
            tta: self.options.tta,
            ..Default::default()
        };
        let expected = engine::target_size(width, height, &settings)?;
        if expected != (self.output_size.width, self.output_size.height) {
            return Err(error(
                "Requested outputSize must match the selected scale and source dimensions.",
            ));
        }
        Ok(settings)
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--job")) {
        return Err(error("Expected --job <request.json>"));
    }
    let path = args.next().ok_or_else(|| error("Missing job request"))?;
    if args.next().is_some() {
        return Err(error("Unexpected worker argument"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16385)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16384 {
        return Err(error("Job request exceeds 16 KiB"));
    }
    let job: Job = serde_json::from_slice(&bytes)?;
    let settings = job.settings()?;
    let cancel = Arc::new(AtomicBool::new(false));
    let listener = Arc::clone(&cancel);
    // Any data or EOF is a stop request. Do not join: a successful worker exits while
    // the host still owns stdin. Rust process exit ends this blocked reader.
    std::thread::Builder::new()
        .name("upscale-host-lifetime".into())
        .spawn(move || {
            let mut byte = [0u8; 1];
            let _ = std::io::stdin().read(&mut byte);
            listener.store(true, Ordering::Relaxed);
        })?;
    engine::upscale(
        &job.input,
        &job.output,
        &settings,
        &cancel,
        &|progress, message| {
            eprintln!(
                "{}",
                serde_json::json!({"progress":progress,"message":message})
            );
        },
    )?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
