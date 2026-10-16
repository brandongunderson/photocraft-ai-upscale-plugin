#![deny(unsafe_code)]
use photocraft_plugin_sdk::api::{
    ApiError, CommandInvocation, ErrorCode, EventRequest, PanelResponse, Result,
};
use photocraft_plugin_sdk::serde_json::{Value, json};

pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
#[cfg(test)]
const MANIFEST: &str = bindings::MANIFEST_JSON;

fn reply(request: &EventRequest, message: &str) -> Result<PanelResponse> {
    Ok(PanelResponse {
        state: request.state.clone(),
        message: message.into(),
        ..PanelResponse::default()
    })
}

fn effect(state: Value, message: &str, command: &str, params: Value) -> Result<PanelResponse> {
    Ok(PanelResponse {
        state,
        message: message.into(),
        commands: vec![CommandInvocation {
            command: command.into(),
            params: params.as_object().cloned().ok_or_else(|| {
                ApiError::new(ErrorCode::InvalidJson, "Invalid processing options.")
            })?,
        }],
        ..PanelResponse::default()
    })
}

fn handle_event(request: &EventRequest) -> Result<PanelResponse> {
    if !request.document.is_object() {
        return reply(request, "Open an image before using AI Upscale.");
    }
    if request.event.as_str() != "upscale" {
        return reply(request, "Unknown action");
    }
    let values = &request.values;
    let scale = match values.get("scale").and_then(Value::as_str) {
        Some("2×") => 2,
        Some("3×") => 3,
        Some("4×") => 4,
        _ => 1,
    };
    let document = &request.document;
    let Some((width, height)) = document["width"].as_u64().zip(document["height"].as_u64()) else {
        return reply(request, "The active image dimensions are unavailable.");
    };
    let Some(output) = width.checked_mul(scale).zip(height.checked_mul(scale)) else {
        return reply(request, "The requested dimensions are too large.");
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
        return reply(
            request,
            "Choose a smaller scale or crop the image. AI Upscale accepts a 100-megapixel source, up to 256-megapixel output, 32,768 pixels per side and an estimated 8 GiB working set.",
        );
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
        .state
        .get("runs")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    let mut state = request.state.as_object().cloned().unwrap_or_default();
    state.insert("runs".into(), json!(runs));
    effect(
        json!(state),
        "Generating an upscale preview…",
        "plugin.processImage",
        json!({
            "processor":"native", "width":output.0,"height":output.1,"preview":true,"options":{
                "engine":engine,"model":model,"scale":scale,"detail":values.get("detail").and_then(Value::as_u64).unwrap_or(100),
                "tile_size":128,"tta":values.get("tta").and_then(Value::as_bool).unwrap_or(false)
            }
        }),
    )
}

photocraft_plugin_sdk::export_plugin!(bindings::MANIFEST_JSON, handle_event);

#[cfg(test)]
mod tests {
    use super::*;
    use photocraft_plugin_sdk::serde_json;

    // Existing feature cases also pass through the actual API-2 SDK request/response boundary.
    fn handle(value: &Value) -> Value {
        let mut input = value.clone();
        if let Some(object) = input.as_object_mut() {
            object.entry("apiVersion").or_insert(json!(2));
            object
                .entry("host")
                .or_insert(json!({"version":1,"capabilities":[]}));
        }
        let input = serde_json::to_vec(&input).unwrap();
        let mut guest = photocraft_plugin_sdk::GuestBuffers::new();
        let buffer = guest.allocate(input.len()).unwrap();
        buffer.copy_from_slice(&input);
        let pointer = buffer.as_ptr();
        match guest.respond(pointer, input.len(), handle_event) {
            Ok(bytes) => serde_json::from_slice(bytes).unwrap(),
            Err(error) => {
                json!({"state": value.get("state").unwrap_or(&Value::Null), "message":error.message,"commands":[]})
            }
        }
    }

    #[test]
    fn api2_metadata_is_additive_and_legacy_envelopes_are_rejected() {
        assert_eq!(photocraft_guest_api_version(), 2);
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["apiVersion"], 2);
        assert_eq!(
            manifest["capabilities"]["required"],
            json!([{"id":"host.commands.invoke","minVersion":1}])
        );
        let base = json!({"apiVersion":2,"host":{"version":1,"capabilities":[]},
            "event":"init","values":{},"document":null,"state":{"saved":7},
            "compatibility":{"panelApi":2,"hostApi":1,"negotiated":{"available":[{"id":"host.commands.invoke","version":1}],"unavailableOptional":[]}},
            "futureHostMetadata":{"added":true}});
        EventRequest::parse(&serde_json::to_vec(&base).unwrap()).unwrap();
        let response = handle(&base);
        assert_eq!(response["state"], base["state"]);
        assert_eq!(response["commands"], json!([]));
        for event in [
            "init",
            "change",
            "document_changed",
            "host_response",
            "unrecognized",
        ] {
            let mut input = base.clone();
            input["event"] = json!(event);
            assert_eq!(handle(&input)["commands"], json!([]));
        }
        let mut old = base.clone();
        old["apiVersion"] = json!(1);
        assert!(EventRequest::parse(&serde_json::to_vec(&old).unwrap()).is_err());
        let old = json!({"event":"init","values":{},"state":{}});
        assert!(EventRequest::parse(&serde_json::to_vec(&old).unwrap()).is_err());
    }

    #[test]
    fn package_panel_and_action_share_the_generic_permission() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["id"], "org.photocraft.ai-upscale");
        assert_eq!(manifest["version"], "2.1.0");
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
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            handle(&json!({"event":"unknown","document":{}}))
                .get("commands")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
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
            assert!(
                handle(&request(width, height))
                    .get("commands")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(
            handle(&json!({"event":"upscale","document":{}}))
                .get("commands")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}
