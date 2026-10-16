use image::{ImageDecoder, ImageEncoder, Rgba, RgbaImage};
use photocraft_ai_upscale_plugin::engine::{self, EngineKind, Settings};
use std::sync::atomic::{AtomicBool, Ordering};

fn fixture(path: &std::path::Path, w: u32, h: u32) {
    let image = RgbaImage::from_fn(w, h, |x, y| {
        Rgba([
            (x % 255) as u8,
            (y % 255) as u8,
            100,
            if x == 0 { 0 } else { 180 },
        ])
    });
    image.save(path).unwrap();
}

#[test]
fn software_output_has_exact_size_alpha_and_srgb_profile() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    let out = dir.path().join("result.png");
    fixture(&input, 17, 11);
    let s = Settings {
        engine: EngineKind::Software,
        width: Some(51),
        ..Default::default()
    };
    let before = std::fs::read(&input).unwrap();
    let report = engine::upscale(&input, &out, &s, &AtomicBool::new(false), &|_, _| {}).unwrap();
    assert_eq!(report.output_size, [51, 33]);
    assert_eq!(report.engine, "lanczos-software");
    assert_eq!(std::fs::read(&input).unwrap(), before);
    let img = image::open(&out).unwrap().to_rgba8();
    let original = image::open(&input).unwrap().to_rgba8();
    let resized = engine::resize_alpha_safe(&original, 51, 33);
    assert!(
        img.pixels()
            .zip(resized.pixels())
            .all(|(a, b)| a.0[3] == b.0[3])
    );
    let mut decoder = image::ImageReader::open(&out)
        .unwrap()
        .into_decoder()
        .unwrap();
    assert!(decoder.icc_profile().unwrap().is_some());
}

#[test]
fn errors_do_not_overwrite_or_silently_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    fixture(&input, 3, 5);
    let output = dir.path().join("existing.png");
    std::fs::write(&output, b"keep me").unwrap();
    let s = Settings {
        engine: EngineKind::Software,
        ..Default::default()
    };
    assert!(engine::upscale(&input, &output, &s, &AtomicBool::new(false), &|_, _| {}).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), b"keep me");
    let missing = Settings {
        runtime: dir.path().join("missing"),
        ..Default::default()
    };
    let output = dir.path().join("missing-ai.png");
    assert!(
        engine::upscale(
            &input,
            &output,
            &missing,
            &AtomicBool::new(false),
            &|_, _| {}
        )
        .is_err()
    );
    assert!(!output.exists());
    let output = dir.path().join("cancelled.png");
    assert!(engine::upscale(&input, &output, &s, &AtomicBool::new(true), &|_, _| {}).is_err());
    assert!(!output.exists());
}

#[test]
fn default_and_missing_settings_enhance_at_original_size_and_preserve_explicit_scales() {
    for settings in [
        Settings::default(),
        serde_json::from_value(serde_json::json!({})).unwrap(),
    ] {
        assert_eq!(settings.scale, 1);
        assert_eq!(
            engine::target_size(4240, 2832, &settings).unwrap(),
            (4240, 2832)
        );
    }
    for scale in 1..=4 {
        let settings: Settings =
            serde_json::from_value(serde_json::json!({"scale": scale})).unwrap();
        assert_eq!(settings.scale, scale);
        assert_eq!(
            engine::target_size(17, 11, &settings).unwrap(),
            (17 * scale, 11 * scale)
        );
    }
}

#[test]
fn sizing_rejects_invalid_and_excessive_allocations() {
    for s in [
        Settings {
            scale: 16,
            ..Default::default()
        },
        Settings {
            width: Some(0),
            ..Default::default()
        },
        Settings {
            detail: 101,
            ..Default::default()
        },
        Settings {
            tile_size: 1,
            ..Default::default()
        },
    ] {
        assert!(engine::target_size(100, 100, &s).is_err());
    }
    assert!(engine::target_size(u32::MAX, 2, &Settings::default()).is_err());
    assert!(
        engine::target_size(
            10000,
            10000,
            &Settings {
                scale: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        engine::target_size(
            19,
            7,
            &Settings {
                width: Some(53),
                ..Default::default()
            }
        )
        .unwrap(),
        (53, 20)
    );
}

#[test]
fn same_size_and_reported_four_times_photo_fit_the_bounded_output_policy() {
    let same = Settings {
        scale: 1,
        ..Default::default()
    };
    assert_eq!(
        engine::target_size(4240, 2832, &same).unwrap(),
        (4240, 2832)
    );
    let four = Settings {
        scale: 4,
        ..Default::default()
    };
    if cfg!(target_pointer_width = "64") {
        const {
            assert!(
                !cfg!(target_pointer_width = "64")
                    || engine::MAX_INPUT_PIXELS * 8 <= engine::MAX_DECODE_BYTES
            );
        }
        assert_eq!(
            engine::target_size(4240, 2832, &four).unwrap(),
            (16960, 11328)
        );
    } else {
        assert!(engine::target_size(4240, 2832, &four).is_err());
    }
    assert!(engine::target_size(8193, 1, &four).is_err());
    assert!(engine::target_size(8000, 8000, &four).is_err());
}

#[test]
fn row_cached_premultiplied_lanczos_matches_full_float_resampling() {
    let source = RgbaImage::from_fn(17, 11, |x, y| {
        Rgba([(x * 15) as u8, (y * 23) as u8, 97, ((x + y) * 9) as u8])
    });
    let mut premultiplied = image::DynamicImage::ImageRgba8(source.clone()).to_rgba32f();
    for pixel in premultiplied.pixels_mut() {
        for channel in 0..3 {
            pixel.0[channel] *= pixel.0[3];
        }
    }
    let mut expected = image::imageops::resize(
        &premultiplied,
        51,
        33,
        image::imageops::FilterType::Lanczos3,
    );
    for pixel in expected.pixels_mut() {
        let alpha = pixel.0[3].clamp(0.0, 1.0);
        for channel in 0..3 {
            pixel.0[channel] = if alpha > 0.00001 {
                (pixel.0[channel] / alpha).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        pixel.0[3] = alpha;
    }
    let expected = image::DynamicImage::ImageRgba32F(expected).to_rgba8();
    let actual = engine::resize_alpha_safe(&source, 51, 33);
    for (actual, expected) in actual.pixels().zip(expected.pixels()) {
        assert!(
            actual
                .0
                .iter()
                .zip(expected.0)
                .all(|(a, b)| a.abs_diff(b) <= 1),
            "{actual:?} differs from {expected:?}"
        );
    }
    assert_eq!(engine::resize_alpha_safe(&source, 17, 11), source);
}

#[test]
fn alpha_resampling_does_not_introduce_hidden_blue_fringe() {
    let mut img = RgbaImage::new(2, 1);
    img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    img.put_pixel(1, 0, Rgba([0, 0, 255, 0]));
    let resized = engine::resize_alpha_safe(&img, 8, 4);
    assert!(
        resized
            .pixels()
            .filter(|p| p.0[3] > 10)
            .all(|p| p.0[2] == 0)
    );
}

#[test]
fn corrupt_embedded_profile_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bad-profile.png");
    let mut encoder = image::codecs::png::PngEncoder::new(std::fs::File::create(&input).unwrap());
    encoder
        .set_icc_profile(b"invalid profile".to_vec())
        .unwrap();
    encoder
        .write_image(&[255, 0, 0], 1, 1, image::ExtendedColorType::Rgb8)
        .unwrap();
    assert!(engine::read_image(&input).is_err());
}

#[test]
fn neural_tiles_align_at_arbitrary_width_and_restore_alpha() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = dir.path().join("runtime");
    std::fs::create_dir_all(runtime.join("bin")).unwrap();
    std::fs::create_dir_all(runtime.join("models")).unwrap();
    let exe = runtime.join("bin").join(if cfg!(windows) {
        "upscayl-bin.exe"
    } else {
        "upscayl-bin"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_mock-ncnn"), exe).unwrap();
    for ext in ["param", "bin"] {
        std::fs::write(
            runtime
                .join("models")
                .join(format!("upscayl-lite-4x.{ext}")),
            b"fixture",
        )
        .unwrap();
    }
    let input = dir.path().join("source.png");
    let output = dir.path().join("output.png");
    RgbaImage::from_pixel(1027, 3, Rgba([120, 80, 50, 190]))
        .save(&input)
        .unwrap();
    let s = Settings {
        runtime,
        model: "upscayl-lite-4x".into(),
        width: Some(2801),
        ..Default::default()
    };
    let report = engine::upscale(&input, &output, &s, &AtomicBool::new(false), &|_, _| {}).unwrap();
    assert_eq!(report.output_size, [2801, 8]);
    assert_eq!(report.engine, "ncnn-vulkan-ai");
    assert!(
        image::open(&output)
            .unwrap()
            .to_rgba8()
            .pixels()
            .all(|p| p.0 == [120, 80, 50, 190])
    );
}

#[test]
fn cancellation_kills_and_reaps_native_process() {
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let other = cancel.clone();
    let signal = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        other.store(true, Ordering::Relaxed);
    });
    let s = upscayl::config::UpscaylOptions {
        model: "sleep".into(),
        ..Default::default()
    };
    let now = std::time::Instant::now();
    let result = upscayl::engine::native::NativeNcnn::execute_cancellable(
        std::path::Path::new(env!("CARGO_BIN_EXE_mock-ncnn")),
        std::path::Path::new("unused"),
        std::path::Path::new("unused"),
        &s,
        None,
        &cancel,
    );
    signal.join().unwrap();
    assert!(result.unwrap_err().to_string().contains("Cancelled"));
    assert!(now.elapsed().as_secs() < 5);
}

fn mock_runtime(dir: &std::path::Path) -> std::path::PathBuf {
    let runtime = dir.join("runtime");
    std::fs::create_dir_all(runtime.join("bin")).unwrap();
    std::fs::create_dir_all(runtime.join("models")).unwrap();
    std::fs::copy(
        env!("CARGO_BIN_EXE_mock-ncnn"),
        runtime.join("bin").join(if cfg!(windows) {
            "upscayl-bin.exe"
        } else {
            "upscayl-bin"
        }),
    )
    .unwrap();
    for ext in ["param", "bin"] {
        std::fs::write(
            runtime
                .join("models")
                .join(format!("upscayl-lite-4x.{ext}")),
            b"fixture",
        )
        .unwrap();
    }
    runtime
}

#[cfg(windows)]
#[test]
fn long_managed_runtime_paths_are_staged_privately_before_legacy_inference() {
    let dir = tempfile::tempdir().unwrap();
    let long = dir
        .path()
        .join("managed-installation-with-a-long-user-selected-location".repeat(3));
    let runtime = mock_runtime(&long);
    assert!(
        runtime
            .join("models/upscayl-lite-4x.param")
            .as_os_str()
            .len()
            > 260
    );
    let input = dir.path().join("source.png");
    let output = dir.path().join("result.png");
    RgbaImage::from_pixel(17, 11, Rgba([120, 80, 50, 190]))
        .save(&input)
        .unwrap();
    let settings = Settings {
        runtime: runtime.clone(),
        model: "upscayl-lite-4x".into(),
        scale: 1,
        ..Default::default()
    };
    let before = std::fs::read(runtime.join("models/upscayl-lite-4x.bin")).unwrap();
    engine::upscale(
        &input,
        &output,
        &settings,
        &AtomicBool::new(false),
        &|_, _| {},
    )
    .unwrap();
    assert_eq!(
        std::fs::read(runtime.join("models/upscayl-lite-4x.bin")).unwrap(),
        before
    );
    let result = image::open(output).unwrap().into_rgba8();
    assert_eq!(result.dimensions(), (17, 11));
    assert!(result.pixels().all(|pixel| pixel.0 == [120, 80, 50, 190]));
}

#[test]
fn enhance_only_reconstructs_at_original_size_without_tile_seams_and_zero_detail_is_original() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    let source = RgbaImage::from_fn(1027, 3, |x, _| {
        Rgba([(x % 251) as u8, 80, 50, ((x % 200) + 55) as u8])
    });
    source.save(&input).unwrap();
    let mut settings = Settings {
        runtime: mock_runtime(dir.path()),
        model: "upscayl-lite-4x".into(),
        scale: 1,
        ..Default::default()
    };
    for detail in [0, 100] {
        settings.detail = detail;
        let output = dir.path().join(format!("same-size-{detail}.png"));
        let report = engine::upscale(
            &input,
            &output,
            &settings,
            &AtomicBool::new(false),
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(report.output_size, [1027, 3]);
        let actual = image::open(output).unwrap().into_rgba8();
        for (x, y, pixel) in actual.enumerate_pixels() {
            assert_eq!(pixel.0[3], source.get_pixel(x, y).0[3]);
            let center = source.get_pixel(x, y).0[0] as f32;
            let expected = if detail == 0 {
                center
            } else {
                let left = source.get_pixel(x.saturating_sub(1), y).0[0] as f32;
                let right = source.get_pixel((x + 1).min(1026), y).0[0] as f32;
                center * 0.75 + (left + right) * 0.125
            };
            assert_eq!(
                pixel.0[0],
                expected.round() as u8,
                "seam or alignment at {x},{y}"
            );
            assert_eq!(&pixel.0[1..3], &[80, 50]);
        }
    }
}

#[test]
fn successful_exit_without_new_tile_output_cannot_reuse_old_pixels() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = mock_runtime(dir.path());
    std::fs::write(runtime.join("bin/fail-second-tile"), b"fixture").unwrap();
    let input = dir.path().join("source.png");
    let output = dir.path().join("output.png");
    RgbaImage::from_fn(1024, 3, |x, _| {
        Rgba([if x < 512 { 30 } else { 220 }, 80, 50, 255])
    })
    .save(&input)
    .unwrap();
    let s = Settings {
        runtime,
        model: "upscayl-lite-4x".into(),
        ..Default::default()
    };
    assert!(engine::upscale(&input, &output, &s, &AtomicBool::new(false), &|_, _| {}).is_err());
    assert!(!output.exists());
}

#[test]
fn neural_inference_does_not_see_hidden_transparent_blue() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = mock_runtime(dir.path());
    let input = dir.path().join("source.png");
    let output = dir.path().join("output.png");
    RgbaImage::from_fn(4, 2, |x, _| {
        if x < 2 {
            Rgba([255, 0, 0, 255])
        } else {
            Rgba([0, 0, 255, 0])
        }
    })
    .save(&input)
    .unwrap();
    let s = Settings {
        runtime,
        model: "upscayl-lite-4x".into(),
        ..Default::default()
    };
    engine::upscale(&input, &output, &s, &AtomicBool::new(false), &|_, _| {}).unwrap();
    assert!(
        image::open(output)
            .unwrap()
            .to_rgba8()
            .pixels()
            .filter(|p| p.0[3] > 10)
            .all(|p| p.0[2] == 0 && p.0[0] == 255)
    );
}
