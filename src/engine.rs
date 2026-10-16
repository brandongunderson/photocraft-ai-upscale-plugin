use crate::{Result, error};
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageReader, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use upscayl::config::{ImageFormat, UpscaylOptions};
use upscayl::engine::native::NativeNcnn;

pub const MODELS: &[(&str, &str)] = &[
    ("upscayl-standard-4x", "Photo · Standard"),
    ("upscayl-lite-4x", "Photo · Fast"),
    ("digital-art-4x", "Illustration · Digital art"),
    ("high-fidelity-4x", "Photo · High fidelity"),
    ("remacri-4x", "Texture · Remacri"),
    ("ultramix-balanced-4x", "Mixed media · Balanced"),
    ("ultrasharp-4x", "Render · UltraSharp"),
];
pub const MAX_PIXELS: u64 = 256_000_000;
pub const MAX_INPUT_PIXELS: u64 = 100_000_000;
pub const MAX_DECODE_BYTES: u64 = if cfg!(target_pointer_width = "64") {
    1024 * 1024 * 1024
} else {
    512 * 1024 * 1024
};
pub const MEMORY_BUDGET: u64 = if cfg!(target_pointer_width = "64") {
    8 * 1024 * 1024 * 1024
} else {
    2 * 1024 * 1024 * 1024
};

fn check_memory(source: u64, target: u64) -> Result<()> {
    let estimated = source
        .saturating_add(target)
        .saturating_mul(32)
        .saturating_add(128 * 1024 * 1024);
    if estimated > MEMORY_BUDGET {
        return Err(error(format!(
            "The estimated working memory exceeds the {} GiB budget. Choose a smaller output width or crop the source.",
            MEMORY_BUDGET / (1024 * 1024 * 1024)
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    Ai,
    Software,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub engine: EngineKind,
    pub model: String,
    pub scale: u32,
    pub width: Option<u32>,
    pub detail: u32,
    pub tile_size: u32,
    pub gpu: Option<i32>,
    pub tta: bool,
    pub runtime: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        let root = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            engine: EngineKind::Ai,
            model: "upscayl-standard-4x".into(),
            scale: 1,
            width: None,
            detail: 100,
            tile_size: 128,
            gpu: None,
            tta: false,
            runtime: root.join("runtime"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub input: PathBuf,
    pub output: PathBuf,
    pub original_size: [u32; 2],
    pub output_size: [u32; 2],
    pub engine: String,
    pub model: Option<String>,
    pub detail: u32,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

pub fn target_size(w: u32, h: u32, s: &Settings) -> Result<(u32, u32)> {
    if w == 0 || h == 0 {
        return Err(error("The image has no pixels."));
    }
    if ![1, 2, 3, 4].contains(&s.scale)
        || s.detail > 100
        || !(s.tile_size == 0 || (32..=1024).contains(&s.tile_size))
        || s.gpu.is_some_and(|v| v < 0)
    {
        return Err(error(
            "Choose a scale of 1, 2, 3, or 4, detail 0–100, tile size 0 or 32–1024, and a nonnegative GPU ID.",
        ));
    }
    let dims = if let Some(width) = s.width {
        let height = (u64::from(h) * u64::from(width) + u64::from(w) / 2) / u64::from(w);
        (width, u32::try_from(height)?.max(1))
    } else {
        (
            w.checked_mul(s.scale)
                .ok_or_else(|| error("Width overflow."))?,
            h.checked_mul(s.scale)
                .ok_or_else(|| error("Height overflow."))?,
        )
    };
    if dims.0 < w
        || dims.1 < h
        || dims.0 == 0
        || dims.0 > 32768
        || dims.1 > 32768
        || u64::from(dims.0) * u64::from(dims.1) > MAX_PIXELS
    {
        return Err(error(
            "Output must preserve or enlarge the image and fit within 32,768 pixels per side and 256 megapixels.",
        ));
    }
    if u64::from(dims.0) > u64::from(w) * 4 || u64::from(dims.1) > u64::from(h) * 4 + 1 {
        return Err(error(
            "AI Upscale supports enlargement up to 4×. Choose a smaller width.",
        ));
    }
    check_memory(
        u64::from(w) * u64::from(h),
        u64::from(dims.0) * u64::from(dims.1),
    )?;
    Ok(dims)
}

pub fn runtime_binary(s: &Settings) -> PathBuf {
    s.runtime.join("bin").join(if cfg!(windows) {
        "upscayl-bin.exe"
    } else {
        "upscayl-bin"
    })
}

pub fn ai_ready(s: &Settings) -> Result<()> {
    if !MODELS.iter().any(|(id, _)| *id == s.model) {
        return Err(error("Unknown AI model."));
    }
    if !runtime_binary(s).is_file() {
        return Err(error(
            "The bundled AI engine is missing. Reinstall the AI Upscale .pcplugin package in PhotoCraft.",
        ));
    }
    if upscayl::models::ModelRegistry::locate_files(&s.runtime.join("models"), &s.model).is_none() {
        return Err(error(format!(
            "Bundled model {} is missing. Reinstall the AI Upscale .pcplugin package in PhotoCraft.",
            s.model
        )));
    }
    Ok(())
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(error("Cancelled. No output was saved."))
    } else {
        Ok(())
    }
}

fn legacy_runtime_path_fits(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        // The bundled NCNN CLI uses 256-wide-character model filename buffers.
        // Leave room for its executable and model-path normalization as well.
        path.as_os_str().encode_wide().count() < 240
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        true
    }
}

/// Isolate the legacy CLI from long managed installation paths. Rust can read those
/// paths safely; the native CLI cannot. Copy only the selected model and fixed runtime
/// files into this job's existing private temporary directory, once for all its tiles.
fn runtime_for_job(s: &Settings, work: &Path, cancel: &AtomicBool) -> Result<Settings> {
    let files = [
        runtime_binary(s),
        s.runtime.join("models").join(format!("{}.param", s.model)),
        s.runtime.join("models").join(format!("{}.bin", s.model)),
    ];
    if files.iter().all(|path| legacy_runtime_path_fits(path)) {
        return Ok(s.clone());
    }
    let mut staged = s.clone();
    staged.runtime = work.join("runtime");
    let destinations = [
        runtime_binary(&staged),
        staged
            .runtime
            .join("models")
            .join(format!("{}.param", s.model)),
        staged
            .runtime
            .join("models")
            .join(format!("{}.bin", s.model)),
    ];
    if destinations
        .iter()
        .any(|path| !legacy_runtime_path_fits(path))
    {
        return Err(error(
            "The temporary folder path is too long for the bundled AI runtime. Use a shorter temporary folder and retry; no output was saved.",
        ));
    }
    std::fs::create_dir_all(staged.runtime.join("bin"))?;
    std::fs::create_dir_all(staged.runtime.join("models"))?;
    for (source, destination) in files.iter().zip(destinations) {
        cancelled(cancel)?;
        std::fs::copy(source, destination)?;
    }
    #[cfg(windows)]
    {
        let library = s.runtime.join("bin/vcomp140.dll");
        if library.is_file() {
            cancelled(cancel)?;
            std::fs::copy(library, staged.runtime.join("bin/vcomp140.dll"))?;
        }
    }
    cancelled(cancel)?;
    Ok(staged)
}

pub fn read_image(path: &Path) -> Result<DynamicImage> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let (w, h) = image::image_dimensions(path)?;
    check_memory(u64::from(w) * u64::from(h), 0)?;
    if u64::from(w) * u64::from(h) > MAX_INPUT_PIXELS {
        return Err(error("Input exceeds 100 megapixels."));
    }
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let profile = decoder.icc_profile()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    if matches!(
        image.color(),
        image::ColorType::Rgb32F | image::ColorType::Rgba32F
    ) {
        return Err(error(
            "HDR/float input needs a tone-mapped sRGB copy before upscaling.",
        ));
    }
    if let Some(bytes) = profile {
        let source = moxcms::ColorProfile::new_from_slice(&bytes)?;
        let target = moxcms::ColorProfile::new_srgb();
        let (layout, pixels) = match source.color_space {
            moxcms::DataColorSpace::Rgb => (moxcms::Layout::Rgb, image.to_rgb8().into_raw()),
            moxcms::DataColorSpace::Gray => (moxcms::Layout::Gray, image.to_luma8().into_raw()),
            _ => {
                return Err(error(
                    "This image's ICC profile is not RGB or grayscale. Export an sRGB copy first.",
                ));
            }
        };
        let transform = source.create_transform_8bit(
            layout,
            &target,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )?;
        let mut converted = vec![0u8; w as usize * h as usize * 3];
        transform.transform(&pixels, &mut converted)?;
        let mut rgba = image.to_rgba8();
        for (p, rgb) in rgba.pixels_mut().zip(converted.as_chunks::<3>().0) {
            p.0[..3].copy_from_slice(rgb);
        }
        image = DynamicImage::ImageRgba8(rgba);
    }
    image.apply_orientation(orientation);
    Ok(image)
}

/// NCNN's own tiles bound GPU memory. Outer tiles also bound CPU intermediates for a 4× model.
/// Keep a 32-pixel context margin and copy only the tile interior, avoiding unreliable borders.
fn neural(
    source: &RgbaImage,
    size: (u32, u32),
    s: &Settings,
    work: &Path,
    cancel: &AtomicBool,
    progress: &dyn Fn(f32, &str),
) -> Result<RgbaImage> {
    ai_ready(s)?;
    let (w, h) = source.dimensions();
    let (tw, th) = size;
    let mut result = RgbaImage::new(tw, th);
    let block = 512u32;
    let count = w.div_ceil(block) * h.div_ceil(block);
    let mut finished = 0u32;
    for y in (0..h).step_by(block as usize) {
        for x in (0..w).step_by(block as usize) {
            cancelled(cancel)?;
            let cw = block.min(w - x);
            let ch = block.min(h - y);
            let left = x.saturating_sub(32);
            let top = y.saturating_sub(32);
            let right = (x + cw + 32).min(w);
            let bottom = (y + ch + 32).min(h);
            let tile =
                image::imageops::crop_imm(source, left, top, right - left, bottom - top).to_image();
            let input = work.join("tile-input.png");
            let output = work.join("tile-output.png");
            alpha_extended_rgb(&tile).save(&input)?;
            // Native code can exit successfully without writing an image. Never reuse a prior tile.
            if output.exists() {
                std::fs::remove_file(&output)?;
            }
            let options = UpscaylOptions {
                model: s.model.clone(),
                scale: Some(4),
                tile_size: Some(s.tile_size),
                gpu_id: s.gpu,
                tta_mode: s.tta,
                models_path: Some(s.runtime.join("models")),
                format: ImageFormat::Png,
                ..Default::default()
            };
            NativeNcnn::execute_cancellable(
                &runtime_binary(s),
                &input,
                &output,
                &options,
                Some(&|pct| {
                    progress(
                        10.0 + 75.0 * (finished as f32 + pct.clamp(0.0, 100.0) / 100.0)
                            / count as f32,
                        "Reconstructing detail…",
                    )
                }),
                cancel,
            )?;
            cancelled(cancel)?;
            let up = read_image(&output)?.to_rgba8();
            if up.dimensions() != ((right - left) * 4, (bottom - top) * 4) {
                return Err(error("The AI runtime returned unexpected dimensions."));
            }
            // Global pixel centers keep arbitrary sizes aligned across tile boundaries.
            let dx0 = (u64::from(x) * u64::from(tw) / u64::from(w)) as u32;
            let dy0 = (u64::from(y) * u64::from(th) / u64::from(h)) as u32;
            let dx1 = (u64::from(x + cw) * u64::from(tw) / u64::from(w)) as u32;
            let dy1 = (u64::from(y + ch) * u64::from(th) / u64::from(h)) as u32;
            copy_resampled(
                &up,
                &mut result,
                [dx0, dy0, dx1, dy1],
                (left, top),
                (w, h),
                cancel,
            )?;
            finished += 1;
        }
    }
    Ok(result)
}

/// Triangle downsampling with globally aligned, precomputed taps. Context pixels keep seams out.
fn copy_resampled(
    tile: &RgbaImage,
    dest: &mut RgbaImage,
    rect: [u32; 4],
    origin: (u32, u32),
    source_size: (u32, u32),
    cancel: &AtomicBool,
) -> Result<()> {
    fn taps(
        start: u32,
        end: u32,
        source: u32,
        target: u32,
        offset: u32,
        extent: u32,
    ) -> Vec<Vec<(u32, f32)>> {
        let step = source as f32 * 4.0 / target as f32;
        let radius = step.max(1.0);
        (start..end)
            .map(|p| {
                let center = (p as f32 + 0.5) * step - 0.5 - offset as f32 * 4.0;
                let mut t = Vec::new();
                let mut sum = 0.0;
                for n in (center - radius).ceil() as i32..=(center + radius).floor() as i32 {
                    let weight = (1.0 - (n as f32 - center).abs() / radius).max(0.0);
                    if weight > 0.0 {
                        t.push((n.clamp(0, extent as i32 - 1) as u32, weight));
                        sum += weight;
                    }
                }
                for (_, weight) in &mut t {
                    *weight /= sum.max(f32::EPSILON);
                }
                t
            })
            .collect()
    }
    let xt = taps(
        rect[0],
        rect[2],
        source_size.0,
        dest.width(),
        origin.0,
        tile.width(),
    );
    let yt = taps(
        rect[1],
        rect[3],
        source_size.1,
        dest.height(),
        origin.1,
        tile.height(),
    );
    for (iy, ys) in yt.iter().enumerate() {
        cancelled(cancel)?;
        for (ix, xs) in xt.iter().enumerate() {
            let mut rgb = [0f32; 3];
            for &(y, wy) in ys {
                for &(x, wx) in xs {
                    let p = tile.get_pixel(x, y);
                    for (sum, value) in rgb.iter_mut().zip(p.0.iter()) {
                        *sum += *value as f32 * wx * wy;
                    }
                }
            }
            dest.put_pixel(
                rect[0] + ix as u32,
                rect[1] + iy as u32,
                image::Rgba([
                    rgb[0].round() as u8,
                    rgb[1].round() as u8,
                    rgb[2].round() as u8,
                    255,
                ]),
            );
        }
    }
    Ok(())
}

pub fn resize_alpha_safe(source: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    resize_alpha_rows(source, w, h, &|| false)
}

/// Premultiplied Lanczos with only the required horizontal float rows cached. A large
/// output never allocates full-frame float source, intermediate, or destination images.
/// The caller checks its cancellation flag immediately after a partial resize returns.
fn resize_alpha_rows(source: &RgbaImage, w: u32, h: u32, stop: &dyn Fn() -> bool) -> RgbaImage {
    if source.dimensions() == (w, h) {
        return source.clone();
    }
    fn axis_taps(source: u32, target: u32) -> Vec<Vec<(u32, f32)>> {
        let ratio = source as f32 / target as f32;
        let scale = ratio.max(1.0);
        let radius = 3.0 * scale;
        (0..target)
            .map(|p| {
                let center = (p as f32 + 0.5) * ratio - 0.5;
                let mut taps = Vec::new();
                let mut total = 0.0;
                let left = ((center - radius).ceil() as i32).max(0);
                let right = ((center + radius).floor() as i32).min(source as i32 - 1);
                for sample in left..=right {
                    let distance = (sample as f32 - center) / scale;
                    let sinc = |x: f32| {
                        if x.abs() < f32::EPSILON {
                            1.0
                        } else {
                            let x = x * std::f32::consts::PI;
                            x.sin() / x
                        }
                    };
                    let weight = sinc(distance) * sinc(distance / 3.0);
                    taps.push((sample as u32, weight));
                    total += weight;
                }
                for (_, weight) in &mut taps {
                    *weight /= total;
                }
                taps
            })
            .collect()
    }
    let mut resized = RgbaImage::new(w, h);
    if w == 0 || h == 0 || source.width() == 0 || source.height() == 0 {
        return resized;
    }
    let horizontal = axis_taps(source.width(), w);
    let vertical = axis_taps(source.height(), h);
    let mut rows: Vec<(u32, Vec<[f32; 4]>)> = Vec::new();
    for (y, taps) in vertical.iter().enumerate() {
        if stop() {
            return resized;
        }
        rows.retain(|(row, _)| taps.iter().any(|(sample, _)| sample == row));
        for &(sample, _) in taps {
            if rows.iter().any(|(row, _)| *row == sample) {
                continue;
            }
            let row = horizontal
                .iter()
                .map(|taps| {
                    let mut pixel = [0.0; 4];
                    for &(x, weight) in taps {
                        let value = source.get_pixel(x, sample).0;
                        let alpha = value[3] as f32;
                        for channel in 0..3 {
                            pixel[channel] += value[channel] as f32 * alpha / 255.0 * weight;
                        }
                        pixel[3] += alpha * weight;
                    }
                    pixel
                })
                .collect();
            rows.push((sample, row));
        }
        for x in 0..w {
            let mut pixel = [0.0; 4];
            for &(sample, weight) in taps {
                if let Some((_, row)) = rows.iter().find(|(index, _)| *index == sample) {
                    for (sum, value) in pixel.iter_mut().zip(row[x as usize]) {
                        *sum += value * weight;
                    }
                }
            }
            let alpha = pixel[3].clamp(0.0, 255.0);
            let mut value = [0; 4];
            if alpha > 0.00255 {
                for channel in 0..3 {
                    value[channel] =
                        (pixel[channel] * 255.0 / alpha).round().clamp(0.0, 255.0) as u8;
                }
            }
            value[3] = alpha.round() as u8;
            resized.put_pixel(x, y as u32, image::Rgba(value));
        }
    }
    resized
}

/// Extend nearest visible RGB into fully transparent pixels before inference. Work is per tile.
fn alpha_extended_rgb(source: &RgbaImage) -> image::RgbImage {
    let (w, h) = source.dimensions();
    let mut rgb = image::RgbImage::new(w, h);
    let mut visited = vec![false; w as usize * h as usize];
    let mut queue = std::collections::VecDeque::new();
    for (x, y, p) in source.enumerate_pixels() {
        if p.0[3] > 0 {
            rgb.put_pixel(x, y, image::Rgb([p.0[0], p.0[1], p.0[2]]));
            visited[y as usize * w as usize + x as usize] = true;
            queue.push_back((x, y));
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let color = *rgb.get_pixel(x, y);
        for (nx, ny) in [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1).filter(|n| *n < w), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1).filter(|n| *n < h)),
        ] {
            if let (Some(nx), Some(ny)) = (nx, ny) {
                let index = ny as usize * w as usize + nx as usize;
                if !visited[index] {
                    visited[index] = true;
                    rgb.put_pixel(nx, ny, color);
                    queue.push_back((nx, ny));
                }
            }
        }
    }
    rgb
}

/// The conventional path retains its small unsharp mask, without cloning and resizing
/// the already resized baseline again. Row checks also make software cancellation prompt.
fn sharpen_conventional(source: &RgbaImage, cancel: &AtomicBool) -> Result<RgbaImage> {
    let (w, h) = source.dimensions();
    let mut result = RgbaImage::new(w, h);
    for y in 0..h {
        cancelled(cancel)?;
        for x in 0..w {
            let mut sum = [0.0; 3];
            let mut count = 0.0;
            for sy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for sx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let sample = source.get_pixel(sx, sy);
                    for (total, value) in sum.iter_mut().zip(sample.0) {
                        *total += value as f32;
                    }
                    count += 1.0;
                }
            }
            let mut pixel = *source.get_pixel(x, y);
            for (channel, total) in pixel.0.iter_mut().zip(sum) {
                let value = *channel as f32;
                *channel = (value + (value - total / count) * 0.4).clamp(0.0, 255.0) as u8;
            }
            result.put_pixel(x, y, pixel);
        }
    }
    Ok(result)
}

pub fn upscale(
    input: &Path,
    output: &Path,
    s: &Settings,
    cancel: &AtomicBool,
    progress: &dyn Fn(f32, &str),
) -> Result<Report> {
    let start = Instant::now();
    cancelled(cancel)?;
    if output.exists() {
        return Err(error("Output already exists. Choose a new file name."));
    }
    if s.engine == EngineKind::Ai {
        ai_ready(s)?;
    }
    if output
        .extension()
        .and_then(|s| s.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case("png"))
    {
        return Err(error("Choose a .png output to preserve transparency."));
    }
    progress(2.0, "Reading image…");
    let img = read_image(input)?;
    let (w, h) = (img.width(), img.height());
    let (tw, th) = target_size(w, h, s)?;
    let source = img.into_rgba8();
    let work = tempfile::tempdir()?;
    progress(8.0, "Preparing image…");
    let baseline = resize_alpha_rows(&source, tw, th, &|| cancel.load(Ordering::Relaxed));
    cancelled(cancel)?;
    let mut enhanced = match s.engine {
        EngineKind::Ai => {
            let runtime = runtime_for_job(s, work.path(), cancel)?;
            neural(&source, (tw, th), &runtime, work.path(), cancel, progress)?
        }
        EngineKind::Software => {
            progress(35.0, "Resizing with Lanczos…");
            sharpen_conventional(&baseline, cancel)?
        }
    };
    progress(88.0, "Preserving transparency…");
    // Alpha is geometry, not model texture. Models always see RGB; restore the resized alpha.
    let weight = s.detail as f32 / 100.0;
    for (n, (p, b)) in enhanced.pixels_mut().zip(baseline.pixels()).enumerate() {
        if n % 65536 == 0 {
            cancelled(cancel)?;
        }
        for c in 0..3 {
            p.0[c] = (b.0[c] as f32 + (p.0[c] as f32 - b.0[c] as f32) * weight)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
        p.0[3] = b.0[3];
    }
    cancelled(cancel)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    progress(94.0, "Saving PNG…");
    let mut encoder = image::codecs::png::PngEncoder::new(std::fs::File::create(temp.path())?);
    encoder.set_icc_profile(moxcms::ColorProfile::new_srgb().encode()?)?;
    encoder.write_image(enhanced.as_raw(), tw, th, image::ExtendedColorType::Rgba8)?;
    cancelled(cancel)?;
    temp.persist_noclobber(output)?;
    progress(100.0, "Complete");
    Ok(Report { input: input.into(), output: output.into(), original_size: [w,h], output_size: [tw,th],
        engine: match s.engine { EngineKind::Ai => "ncnn-vulkan-ai", EngineKind::Software => "lanczos-software" }.into(),
        model: (s.engine == EngineKind::Ai).then(|| s.model.clone()), detail: s.detail,
        duration_ms: start.elapsed().as_millis() as u64,
        warnings: vec!["Output is an 8-bit sRGB PNG. Source layers, EXIF and original bit depth are not retained.".into()] })
}
