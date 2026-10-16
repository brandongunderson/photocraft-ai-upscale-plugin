# AI Upscale plugin for PhotoCraft

AI Upscale runs local image enhancement and enlargement inside PhotoCraft. Install **PhotoCraft-AI-Upscale.pcplugin**, then open **Plugins > AI Upscale**. The package includes the panel, processing engine, and seven AI models.

[Source repository](https://github.com/brandongunderson/photocraft-ai-upscale-plugin) · [Changelog](CHANGELOG.md)

[Download the latest plugin](https://github.com/brandongunderson/photocraft-ai-upscale-plugin/releases/latest/download/PhotoCraft-AI-Upscale.pcplugin) · [All releases](https://github.com/brandongunderson/photocraft-ai-upscale-plugin/releases)

Version **2.1.1** integrates through the PhotoCraft Plugin API on Windows x64. The package owns its models, settings and inference dependencies; PhotoCraft supplies the panel, progress, comparison preview and editing controls. This is a PhotoCraft plugin; Adobe UXP, CEP and `.8BF` hosts are not supported.

## Install and use

1. In the current PhotoCraft build, choose **Plugins > Manage Plugins… > Browse…**.
2. Select **PhotoCraft-AI-Upscale.pcplugin**, review its requested permissions and bundled native engine, then choose **Install plugin** or **Update plugin**.
3. Click **Done**, open an image, then choose **Plugins > AI Upscale**.
4. **1× · Enhance only** is selected by default and preserves the image dimensions. Select **2×**, **3×**, or **4×** when you want a larger result, choose a model and detail strength, then click **Generate preview**. Valid saved scale selections are retained when updating the plugin.
5. PhotoCraft shows progress and a **Cancel** action. When processing finishes, drag the comparison divider to inspect the original and result. Use **Fit**, **100%**, and pan to inspect detail. Generating the preview leaves the document unchanged.
6. Choose the destination and click **Apply**. **New tab** creates a new unsaved result document. **New layer** is available when the result keeps the original dimensions; it inserts an undoable raster layer and preserves the existing layers. Save the document normally.

PhotoCraft manages the installed package and finds its bundled engine automatically. The downloaded `.pcplugin` file can be moved after installation. To disable or remove AI Upscale, use **Manage Plugins**. Installing an update is the same one-file workflow.

## Updating

In **Plugins → Manage Plugins**, select **Check for updates** to check this plugin’s [own GitHub releases](https://github.com/brandongunderson/photocraft-ai-upscale-plugin/releases). When an update is available, select **Update…**, review the permissions, then choose **Update plugin**. Checking and installation are explicit actions; updates are never installed automatically and the installed plugin continues to work offline.

## Models and processing

| Choice | Use |
|---|---|
| Photo · Standard | General photographic detail |
| Photo · Fast | Faster processing using the lighter model |
| Illustration · Digital art | Illustrations and drawn imagery |
| Photo · High Fidelity | Photographs with an emphasis on realistic detail and smooth textures |
| Remacri · Non-commercial | Alternative photo reconstruction emphasizing detail and sharpness |
| UltraMix Balanced · Non-commercial | Alternative photo reconstruction balancing detail and sharpness |
| UltraSharp · Non-commercial | Alternative photo reconstruction emphasizing sharpness |
| Conventional resize (Lanczos) | Explicit software resizing without AI or a Vulkan GPU |

**Remacri, UltraMix Balanced and UltraSharp are non-commercial models.** Their names are marked in the dropdown, and their attribution/license terms are retained in the package. Use the other four models for workflows that need to avoid these non-commercial weights. See [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) for the separate terms of each bundled component; the Rust source's MIT license does not cover all weights.

High Fidelity is Upscayl's alias for Philip Hofmann's **4xHFA2k**, a Real-ESRGAN-based RRDBNet model trained on anime imagery, released under CC BY 4.0. Upscayl presents it as a general image option; compare the preview on your own image rather than assuming it is a dedicated photo-restoration model. Model attribution and pinned upstream records are retained in `runtime/MODEL-PROVENANCE.md`.

AI inference runs locally and requires a compatible Vulkan GPU and driver. An AI error is reported; it never silently switches to conventional resizing. **Extra AI passes** enable the model's slower test-time augmentation. Detail strength blends the reconstructed RGB result with the conventional resized image; transparency is resized separately.

**1× · Enhance only** uses the selected 4× reconstruction model on overlapping tiles, then reduces the reconstructed detail back to the original dimensions. It can alter texture and edges without changing image size. These are super-resolution models; they are not dedicated noise-removal, motion-deblur, face-recovery, or generative restoration models. At 1×, zero detail returns the original pixels after conversion to the output color space. With **Conventional resize**, 1× applies ordinary edge sharpening rather than AI reconstruction.

The plugin receives the full-resolution visible composite of the active document as an **8-bit RGBA sRGB PNG**. The host converts supported 8/16-bit RGB and Grayscale documents before invoking the plugin. Worker output is a flattened **8-bit sRGB** image with transparency. A new tab uses that output and preserves the source document's DPI. For a same-size new layer, the host converts the result into the source document's supported color profile and bit depth before insertion; 8-bit inference does not restore original 16-bit precision. HDR/32-bit and other color modes require converting a copy first.

Input is capped at 100 megapixels. Output must fit within 32,768 pixels per side, 256 megapixels, and the estimated working-memory budget: 8 GiB on a 64-bit host, 2 GiB on a 32-bit build. The estimate is `(source pixels + output pixels) × 32 bytes + 128 MiB`, so it can reject an image below the pixel limit. A 4,240 × 2,832 source at 4× produces 16,960 × 11,328 pixels (192.1 MP), which fits these 64-bit limits. The package itself targets Windows x64. These are bounded acceptance limits rather than a promise that every machine has sufficient free memory or GPU capacity. GPU-memory requirements vary by model. CPU intermediates use overlapping inference tiles and row-cached premultiplied Lanczos resampling instead of full-frame float images.

Private job files are removed by PhotoCraft after success, failure or cancellation. Long managed installation paths are handled by copying the selected model pair and fixed inference runtime into the private job directory before running the legacy CLI; copies are reused for that job's tiles and removed afterward. A generated preview is tied to the source document; changing its contents before Apply invalidates the result. Changing processing settings requires generating a fresh preview. Closing or discarding the comparison does not apply the result.

## How the installed plugin works

The WebAssembly module defines the panel and handles its **Generate preview** button inside PhotoCraft. It computes the requested output dimensions from the source dimensions and selected scale, then invokes the general `plugin.processImage` command with `processor: "native"`, explicit `width`/`height`, `preview: true`, and the plugin's own options object. The host reads the current document, launches this installation's worker, reports progress, and retains the completed image for comparison. The host owns comparison, Apply, new-document creation, and layer insertion. The guest cannot read raw preview pixels or choose arbitrary editor mutation commands.

The bundled native engine runs as a hidden background process managed by PhotoCraft. This keeps GPU inference outside the editor process while all user interaction stays in PhotoCraft. It has no separate window and does not connect over TCP. The engine launches the bundled NCNN executable to perform inference, then exits. Cancellation and loss of the host stop its work and reap the inference process.

Installing this package trusts its native executable as well as its WebAssembly module. The native executable is not restricted by the WASM sandbox. The guest cannot choose arbitrary executables, image paths, or runtime folders: those are resolved by the generic host from the owning installation. All inference dependencies ship in this package, including `upscayl-bin.exe`, `vcomp140.dll`, the seven model pairs and their notices. Windows OS libraries and a compatible GPU driver with its Vulkan loader remain system prerequisites; a separately installed compiler or Visual C++ redistributable is not required by the audited release imports.

## Independent source project

The panel and native worker build independently of the PhotoCraft editor.

```text
authoring.json            Plugin API descriptor and package format 2 metadata
extension/build.rs        Validates authoring and generates manifest/bindings
extension/src/lib.rs      Typed panel handler and bounded SDK export
extension/vendor/         Private PhotoCraft API/SDK sources and release hashes
src/worker.rs             Headless job protocol and host-lifetime cancellation
src/engine.rs             Tiled image processing, alpha handling and color conversion
vendor/upscayl-rs/         Licensed native NCNN wrapper and software resampling
runtime/bin/              Bundled inference executable and its runtime DLL
runtime/models/           Seven bundled model pairs
runtime/source/           Backend source snapshot and provenance
tests/                    Synthetic image, worker and cancellation tests
scripts/build-plugin.ps1  Build and package a single installable .pcplugin
scripts/fetch-assets.ps1  Restore exact runtime/model assets from pinned hashes
```

PhotoCraft's generic plugin loader, permission checks, panel renderer, document access, comparison UI, and `plugin.processImage` bridge live in the PhotoCraft repository. This project's panel, options, model allowlist, sizing policy, inference libraries, model assets and processing implementation ship in the installed package. Changing those plugin details does not require recompiling PhotoCraft while the general API and host resource envelope remain compatible. The host does not identify AI Upscale specially or require fixed upscale asset filenames.

Model selection is validated by this plugin's own worker. This release bundles and exposes `upscayl-standard-4x`, `upscayl-lite-4x`, `digital-art-4x`, `high-fidelity-4x`, `remacri-4x`, `ultramix-balanced-4x`, and `ultrasharp-4x`. Adding a model requires updating this project's panel, worker model list, assets, pinned hashes and licenses, rebuilding the package, and installing the update. No PhotoCraft core change is needed.

## Build and test

Use Rust 1.95 or later with a Windows x64 native toolchain and the `wasm32-unknown-unknown` target, plus Python 3.11 or later for collecting dependency licenses. A normal Rust Windows installation needs its matching linker/build tools. Downloaded inference binaries and model files are excluded from Git. Restore them with `scripts/fetch-assets.ps1`; it uses the fixed URLs and SHA256 hashes in `runtime/downloads.json`. The packaging script verifies those hashes again before packaging.

From this project directory:

```powershell
rustup target add wasm32-unknown-unknown
./scripts/fetch-assets.ps1
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --manifest-path extension/Cargo.toml
./scripts/build-plugin.ps1 -Python python
```

The output is `dist/PhotoCraft-AI-Upscale.pcplugin`. If all dependencies are already cached, pass `-Offline` to the packaging script. `-DebugWorker` creates a package with the debug engine for local investigation. Source consumers do not need a PhotoCraft checkout to build the plugin; running it requires a compatible host. For packaged inference and dependency checks, install Python with Pillow and run `python scripts/smoke-package.py --package dist/PhotoCraft-AI-Upscale.pcplugin --long-path`. Add `--large` to exercise 24-MP same-size and the reported 192-MP four-times output. The smoke checker audits native PE imports and executes with a Windows-only PATH so developer toolchain DLLs cannot conceal missing bundled dependencies.

Use `./scripts/fetch-assets.ps1 -VerifyOnly` to check local assets without downloading. Asset restoration is a developer build step; the installed plugin continues to include all seven models and makes no downloads during use.

To edit the panel, update `authoring.json` and its matching value/event handling in `extension/src/lib.rs`. To edit processing, change `src/engine.rs` and add suitable tests. Rebuild and review the package in Manage Plugins; choose **Update plugin** for a newer version or use **Advanced** for same-version reinstall/repair. The manifest is embedded in the WASM module at build time; editing a loose manifest after installation has no effect. Keep the stable plugin id for updates; bump its version for releases. The version is metadata rather than an automatic update mechanism.

## Repository contents

The native worker and `extension/` panel have separate Cargo workspaces and lockfiles. The local Rust dependency, `vendor/upscayl-rs/`, is included in this repository. Building either workspace requires no sibling project or PhotoCraft checkout.

Source history includes lockfiles, manifests, build scripts, vendored source, licenses and asset provenance. Restore downloaded binaries and model weights from `runtime/downloads.json`. Cargo caches, generated packages, test images and machine-specific validation reports stay local. The release summary in `verification/TEST-RESULTS.md` records the tested package and limits; its referenced JSON reports accompany local test runs.

Distribute the complete `.pcplugin` through GitHub Releases. Keep its stable plugin id when publishing updates, and increase the plugin version for a new release.

## Package and worker contracts

`.pcplugin` is a ZIP archive with `package.json` at its root:

```json
{
  "packageVersion": 2,
  "module": "plugin.pcext",
  "native": {
    "service": "plugin.processImage",
    "platform": "windows-x86_64",
    "entry": "photocraft-upscale-worker.exe"
  }
}
```

The compiled WASM module is `plugin.pcext`. The native executable sits beside `runtime/`; it locates its inference executable and models relative to itself. Preserve that structure. The archive contains no wrapper directory.

The host launches `photocraft-upscale-worker --job <private-request.json>`. The bounded JSON request contains `apiVersion: 1`, absolute `input` and `output` PNG paths, `colorSpace: "srgb"`, `outputSize: {"width": ..., "height": ...}`, and `options` with `engine`, `model`, `scale` (1–4), `detail`, `tile_size`, and `tta`. The host consumes the guest's `preview` flag; it is never forwarded to the worker. The worker validates the API version, color space, RGBA8 PNG source, own model/option limits and exact agreement between `outputSize` and source dimensions multiplied by scale before processing. Unknown fields and runtime-path injection are rejected. The worker emits progress JSON lines to stderr, writes the output PNG only on success, and exits nonzero on failure. Any stdin data or EOF requests cancellation. These details are for host/plugin developers; users never configure them.

## Licenses and redistribution

The plugin's Rust source is MIT. Bundled inference binaries, runtime components and model weights retain their own terms; they are not relicensed by the plugin. Preserve [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md), the licenses under `runtime/`, its checksum/provenance manifests, and the included backend source snapshot when redistributing the package. This is an independent plugin, not an official PhotoCraft or Adobe product.

## PhotoCraft Plugin API compatibility

This plugin uses the **PhotoCraft Plugin API** through the privately bundled SDK 0.1.
The Windows x64 host must support package format 2, `host.commands.invoke` capability version 1,
the `plugin.processImage` command, comparison previews, and the 256-megapixel/8-GiB native
output envelope. The worker uses native image protocol 1. Package and native worker contracts
are versioned independently of the Plugin API.

Edit `authoring.json` for both the panel descriptor and archive metadata. The panel's
build script validates it with the private SDK and generates the embedded descriptor and
bindings into Cargo output. The package builder derives its root `package.json` from the
same source. `extension/vendor/` contains this project's SDK/API sources and their
`SDK-RELEASE.json` hashes. Builds require no PhotoCraft source checkout or sibling plugin.
The SDK handles bounded ABI buffers and Plugin API event validation, including additive host
metadata. Failed/unknown/observer events propose no processing and preserve panel state.
