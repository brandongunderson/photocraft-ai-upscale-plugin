//! Sandboxed AI Upscale panel, built independently of the PhotoCraft host.
//! Request storage stays in a safe Rust buffer; only ABI export attributes need `unsafe`.
use serde_json::{Value, json};
use std::sync::Mutex;

const MANIFEST: &str = include_str!("../manifest.json");
static REQUEST: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[unsafe(no_mangle)]
pub extern "C" fn pce_api_version() -> u32 {
    1
}

fn packed(bytes: &[u8]) -> u64 {
    ((bytes.len() as u64) << 32) | bytes.as_ptr() as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn pce_manifest() -> u64 {
    packed(MANIFEST.as_bytes())
}

#[unsafe(no_mangle)]
pub extern "C" fn pce_alloc(size: u32) -> *mut u8 {
    if size > 256 * 1024 {
        return std::ptr::null_mut();
    }
    let Ok(mut request) = REQUEST.lock() else {
        return std::ptr::null_mut();
    };
    request.resize(size as usize, 0);
    request.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn pce_handle(ptr: *const u8, len: u32) -> u64 {
    let request = REQUEST
        .lock()
        .ok()
        .and_then(|request| {
            if request.as_ptr() != ptr {
                return None;
            }
            request
                .get(..len as usize)
                .and_then(|bytes| serde_json::from_slice(bytes).ok())
        })
        .unwrap_or(Value::Null);
    let response = handle(&request);
    let bytes = serde_json::to_vec(&response)
        .unwrap_or_else(|_| b"{\"message\":\"Could not encode response\"}".to_vec());
    packed(Box::leak(bytes.into_boxed_slice()))
}

fn handle(request: &Value) -> Value {
    if request.get("document").is_none_or(Value::is_null) {
        return json!({"message":"Open an image before using AI Upscale."});
    }
    if request.get("event").and_then(Value::as_str) != Some("upscale") {
        return json!({"message":"Unknown action"});
    }
    let values = request.get("values").unwrap_or(&Value::Null);
    let scale = match values.get("scale").and_then(Value::as_str) {
        Some("2×") => 2,
        Some("3×") => 3,
        Some("4×") => 4,
        _ => 1,
    };
    let document = &request["document"];
    let Some((width, height)) = document["width"].as_u64().zip(document["height"].as_u64()) else {
        return json!({"message":"The active image dimensions are unavailable."});
    };
    let Some(output) = width.checked_mul(scale).zip(height.checked_mul(scale)) else {
        return json!({"message":"The requested dimensions are too large."});
    };
    let source_pixels = width.saturating_mul(height);
    let target_pixels = output.0.saturating_mul(output.1);
    if width == 0
        || height == 0
        || source_pixels > 100_000_000
        || output.0 > 32768
        || output.1 > 32768
        || target_pixels > 256_000_000
        || source_pixels
            .saturating_add(target_pixels)
            .saturating_mul(32)
            .saturating_add(128 * 1024 * 1024)
            > 8 * 1024 * 1024 * 1024
    {
        return json!({"message":"Choose a smaller scale or crop the image. AI Upscale accepts a 100-megapixel source, up to 256-megapixel output, 32,768 pixels per side and an estimated 8 GiB working set."});
    }
    let model = match values.get("model").and_then(Value::as_str) {
        Some("Photo · Fast") => "upscayl-lite-4x",
        Some("Illustration · Digital art") => "digital-art-4x",
        Some("Photo · High Fidelity") => "high-fidelity-4x",
        Some("Remacri · Non-commercial") => "remacri-4x",
        Some("UltraMix Balanced · Non-commercial") => "ultramix-balanced-4x",
        Some("UltraSharp · Non-commercial") => "ultrasharp-4x",
        _ => "upscayl-standard-4x",
    };
    let engine =
        if values.get("engine").and_then(Value::as_str) == Some("Conventional resize (Lanczos)") {
            "software"
        } else {
            "ai"
        };
    let runs = request
        .get("state")
        .and_then(|s| s.get("runs"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    json!({"state":{"runs":runs},"commands":[{"command":"plugin.processImage","params":{
        "processor":"native", "width":output.0,"height":output.1,"preview":true,"options":{
        "engine":engine,"model":model,"scale":scale,"detail":values.get("detail").and_then(Value::as_u64).unwrap_or(100),
        "tile_size":128,"tta":values.get("tta").and_then(Value::as_bool).unwrap_or(false)
    }}}]})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_panel_and_action_share_the_generic_permission() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["id"], "org.photocraft.ai-upscale");
        assert_eq!(manifest["version"], "2.0.0");
        assert_eq!(
            manifest["permissions"]["commands"],
            json!(["plugin.processImage"])
        );
        assert_eq!(
            manifest["panel"]["buttons"],
            json!([{"id":"upscale","label":"Generate preview"}])
        );
        assert_eq!(manifest["panel"]["preview"], true);
        let response = handle(&json!({
            "event":"upscale", "document":{"width":16,"height":12},
            "state":{"runs":7},
            "values":{"scale":"3×","model":"Photo · Fast","detail":64,
                      "engine":"Conventional resize (Lanczos)","tta":true}
        }));
        assert_eq!(response["state"]["runs"], 8);
        assert_eq!(
            response["commands"],
            json!([{
                "command":"plugin.processImage",
                "params":{"processor":"native","width":48,"height":36,"preview":true,
                    "options":{"engine":"software","model":"upscayl-lite-4x","scale":3,
                          "detail":64,"tile_size":128,"tta":true}}
            }])
        );
    }

    #[test]
    fn enhance_only_is_the_panel_and_action_default_and_explicit_scales_are_preserved() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let scale_field = manifest["panel"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["id"] == "scale")
            .unwrap();
        assert_eq!(scale_field["default"], "1× · Enhance only");
        for (value, expected) in [
            (None, 1),
            (Some("1× · Enhance only"), 1),
            (Some("2×"), 2),
            (Some("3×"), 3),
            (Some("4×"), 4),
            (Some("unrecognized old value"), 1),
        ] {
            let mut values = json!({});
            if let Some(value) = value {
                values["scale"] = json!(value);
            }
            let response = handle(
                &json!({"event":"upscale", "document":{"width":16,"height":12}, "values":values}),
            );
            let params = &response["commands"][0]["params"];
            assert_eq!(params["options"]["scale"], expected);
            assert_eq!(params["width"], 16 * expected);
            assert_eq!(params["height"], 12 * expected);
            assert_eq!(params["preview"], true);
            assert_eq!(params["options"]["engine"], "ai");
        }
    }

    #[test]
    fn only_shipped_models_are_exposed_and_missing_document_never_processes() {
        let models = [
            ("Photo · Standard", "upscayl-standard-4x"),
            ("Photo · Fast", "upscayl-lite-4x"),
            ("Illustration · Digital art", "digital-art-4x"),
            ("Photo · High Fidelity", "high-fidelity-4x"),
            ("Remacri · Non-commercial", "remacri-4x"),
            ("UltraMix Balanced · Non-commercial", "ultramix-balanced-4x"),
            ("UltraSharp · Non-commercial", "ultrasharp-4x"),
        ];
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let model_field = manifest["panel"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|field| field["id"] == "model")
            .unwrap();
        assert_eq!(model_field["options"], json!(models.map(|(name, _)| name)));
        for (name, id) in models {
            let response = handle(&json!({
                "event":"upscale", "document":{"width":16,"height":12},
                "values":{"model":name,"scale":"4×","engine":"Local AI (Vulkan)"}
            }));
            assert_eq!(response["commands"][0]["params"]["options"]["model"], id);
            assert_eq!(response["commands"][0]["params"]["options"]["engine"], "ai");
            assert_eq!(response["commands"][0]["params"]["options"]["scale"], 4);
        }
        assert!(
            handle(&json!({"event":"upscale","document":null}))
                .get("commands")
                .is_none()
        );
        assert!(
            handle(&json!({"event":"unknown","document":{}}))
                .get("commands")
                .is_none()
        );
    }

    #[test]
    fn plugin_owns_dimension_limits_and_preserves_large_enlargement_support() {
        let request = |width, height| {
            json!({"event":"upscale",
            "document":{"width":width,"height":height},"values":{"scale":"4×"}})
        };
        let response = handle(&request(4240u64, 2832u64));
        assert_eq!(response["commands"][0]["params"]["width"], 16960);
        assert_eq!(response["commands"][0]["params"]["height"], 11328);
        for (width, height) in [(0, 12), (10000, 10000), (u64::MAX, 1)] {
            assert!(handle(&request(width, height)).get("commands").is_none());
        }
        assert!(
            handle(&json!({"event":"upscale","document":{}}))
                .get("commands")
                .is_none()
        );
    }
}
