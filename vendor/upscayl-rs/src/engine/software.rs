use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

pub fn upscale_software(
    img: &DynamicImage,
    scale: u32,
    custom_width: Option<u32>,
    custom_height: Option<u32>,
) -> RgbaImage {
    let (orig_w, orig_h) = img.dimensions();
    let (target_w, target_h) = match (custom_width, custom_height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let aspect = orig_h as f64 / orig_w as f64;
            (w, (w as f64 * aspect).round() as u32)
        }
        (None, Some(h)) => {
            let aspect = orig_w as f64 / orig_h as f64;
            ((h as f64 * aspect).round() as u32, h)
        }
        (None, None) => (orig_w * scale, orig_h * scale),
    };

    // 1. High-quality Lanczos3 Resampling
    let resized = image::imageops::resize(img, target_w, target_h, FilterType::Lanczos3);

    // 2. Adaptive Edge-Detail Enhancement (Unsharp Masking for super-resolution crispness)
    enhance_details(&resized, 0.4, 1.2)
}

/// Applies an adaptive unsharp mask filter to enhance edge sharpness and texture micro-contrast
fn enhance_details(img: &RgbaImage, amount: f32, radius: f32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = ImageBuffer::new(w, h);

    // Box blur approximation of Gaussian blur for detail difference extraction
    let r = radius.round() as i32;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let orig_px = img.get_pixel(x as u32, y as u32);
            let mut r_sum = 0.0f32;
            let mut g_sum = 0.0f32;
            let mut b_sum = 0.0f32;
            let mut count = 0.0f32;

            for dy in -r..=r {
                let ny = y + dy;
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                for dx in -r..=r {
                    let nx = x + dx;
                    if nx < 0 || nx >= w as i32 {
                        continue;
                    }
                    let p = img.get_pixel(nx as u32, ny as u32);
                    r_sum += p[0] as f32;
                    g_sum += p[1] as f32;
                    b_sum += p[2] as f32;
                    count += 1.0;
                }
            }

            let blur_r = r_sum / count;
            let blur_g = g_sum / count;
            let blur_b = b_sum / count;

            let diff_r = orig_px[0] as f32 - blur_r;
            let diff_g = orig_px[1] as f32 - blur_g;
            let diff_b = orig_px[2] as f32 - blur_b;

            let new_r = (orig_px[0] as f32 + diff_r * amount).clamp(0.0, 255.0) as u8;
            let new_g = (orig_px[1] as f32 + diff_g * amount).clamp(0.0, 255.0) as u8;
            let new_b = (orig_px[2] as f32 + diff_b * amount).clamp(0.0, 255.0) as u8;

            out.put_pixel(x as u32, y as u32, Rgba([new_r, new_g, new_b, orig_px[3]]));
        }
    }

    out
}
