use image::{Rgba, RgbaImage};
use serde_json::json;
use std::{
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

fn start(root: &Path, engine: &str, extra: serde_json::Value) -> Child {
    let exe = root.join(if cfg!(windows) {
        "photocraft-upscale-worker.exe"
    } else {
        "photocraft-upscale-worker"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_photocraft-upscale-worker"), &exe).unwrap();
    let input = root.join("input.png");
    RgbaImage::from_pixel(16, 12, Rgba([120, 80, 40, 170]))
        .save(&input)
        .unwrap();
    let mut options = json!({"engine":engine,"model":"upscayl-lite-4x","scale":2,"detail":100,"tile_size":128,"tta":false});
    options
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let scale = options["scale"].as_u64().unwrap_or(1);
    let request = root.join("job.json");
    std::fs::write(
        &request,
        serde_json::to_vec(
            &json!({"apiVersion":1,"input":input,"output":root.join("output.png"),
        "options":options,"outputSize":{"width":16*scale,"height":12*scale},"colorSpace":"srgb"}),
        )
        .unwrap(),
    )
    .unwrap();
    Command::new(exe)
        .arg("--job")
        .arg(request)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait(child: &mut Child) -> std::process::ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Worker did not terminate promptly");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn software_protocol_preserves_transparency_and_rejects_runtime_injection() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = start(dir.path(), "software", json!({}));
    assert!(wait(&mut child).success());
    let image = image::open(dir.path().join("output.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(image.dimensions(), (32, 24));
    assert!(image.pixels().all(|p| p.0 == [120, 80, 40, 170]));
    std::fs::remove_file(dir.path().join("output.png")).unwrap();
    let mut child = start(
        dir.path(),
        "software",
        json!({"runtime":"arbitrary-program"}),
    );
    assert!(!wait(&mut child).success());
    assert!(!dir.path().join("output.png").exists());
}

#[test]
fn same_size_protocol_is_supported_but_host_preview_metadata_is_not_a_worker_option() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = start(dir.path(), "software", json!({"scale":1}));
    assert!(wait(&mut child).success());
    let output = dir.path().join("output.png");
    assert_eq!(
        image::open(&output).unwrap().into_rgba8().dimensions(),
        (16, 12)
    );
    std::fs::remove_file(&output).unwrap();
    let mut child = start(dir.path(), "software", json!({"scale":1,"preview":true}));
    assert!(!wait(&mut child).success());
    assert!(!output.exists());
}

#[test]
fn host_cancel_or_lost_connection_stops_native_inference() {
    for eof in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("runtime/bin");
        let models = dir.path().join("runtime/models");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&models).unwrap();
        std::fs::copy(
            env!("CARGO_BIN_EXE_mock-ncnn"),
            bin.join(if cfg!(windows) {
                "upscayl-bin.exe"
            } else {
                "upscayl-bin"
            }),
        )
        .unwrap();
        std::fs::write(bin.join("sleep-worker"), b"fixture").unwrap();
        for ext in ["param", "bin"] {
            std::fs::write(models.join(format!("upscayl-lite-4x.{ext}")), b"fixture").unwrap();
        }
        let mut child = start(dir.path(), "ai", json!({}));
        let started = Instant::now();
        while !bin.join("worker-started").exists() && started.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            bin.join("worker-started").exists(),
            "NCNN child must have started"
        );
        let mut stdin = child.stdin.take().unwrap();
        if !eof {
            stdin.write_all(b"cancel\n").unwrap();
        }
        drop(stdin);
        assert!(!wait(&mut child).success());
        assert!(!dir.path().join("output.png").exists());
    }
}

#[test]
fn malformed_protocol_and_oversized_requests_fail_without_output() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("output.png");
    RgbaImage::from_pixel(16, 12, Rgba([120, 80, 40, 170]))
        .save(dir.path().join("input.png"))
        .unwrap();
    let valid = json!({
        "apiVersion":1,"input":dir.path().join("input.png"),"output":output,
        "options":{"engine":"software","model":"upscayl-lite-4x","scale":2,
                   "detail":100,"tile_size":128,"tta":false},
        "outputSize":{"width":32,"height":24},"colorSpace":"srgb"
    });
    let mut wrong_version = valid.clone();
    wrong_version["apiVersion"] = json!(2);
    let mut relative_path = valid.clone();
    relative_path["input"] = json!("relative.png");
    let mut same_paths = valid.clone();
    same_paths["input"] = same_paths["output"].clone();
    let mut unexpected = valid.clone();
    unexpected["executable"] = json!("arbitrary-program");
    let mut wrong_color = valid.clone();
    wrong_color["colorSpace"] = json!("display-p3");
    let mut wrong_size = valid.clone();
    wrong_size["outputSize"]["width"] = json!(31);
    let mut extra_size = valid.clone();
    extra_size["outputSize"]["depth"] = json!(8);
    let mut missing_size = valid.clone();
    missing_size.as_object_mut().unwrap().remove("outputSize");
    let mut bad_model = valid.clone();
    bad_model["options"]["model"] = json!("../../unknown");
    let mut bad_detail = valid.clone();
    bad_detail["options"]["detail"] = json!(101);
    let mut bad_scale = valid;
    bad_scale["options"]["scale"] = json!(5);
    let mut requests: Vec<Vec<u8>> = [
        wrong_version,
        relative_path,
        same_paths,
        unexpected,
        wrong_color,
        wrong_size,
        extra_size,
        missing_size,
        bad_model,
        bad_detail,
        bad_scale,
    ]
    .iter()
    .map(|request| serde_json::to_vec(request).unwrap())
    .collect();
    requests.extend([vec![b' '; 16_385], b"not-json".to_vec()]);
    for bytes in requests {
        let request = dir.path().join("job.json");
        std::fs::write(&request, bytes).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_photocraft-upscale-worker"))
            .arg("--job")
            .arg(&request)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        assert!(!wait(&mut child).success());
        assert!(!output.exists());
    }
}

#[test]
fn generic_protocol_rejects_non_rgba8_input() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = start(dir.path(), "software", json!({}));
    assert!(wait(&mut child).success());
    std::fs::remove_file(dir.path().join("output.png")).unwrap();
    image::RgbImage::new(16, 12)
        .save(dir.path().join("input.png"))
        .unwrap();
    let mut child = Command::new(dir.path().join(if cfg!(windows) {
        "photocraft-upscale-worker.exe"
    } else {
        "photocraft-upscale-worker"
    }))
    .arg("--job")
    .arg(dir.path().join("job.json"))
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .unwrap();
    assert!(!wait(&mut child).success());
    assert!(!dir.path().join("output.png").exists());
}
