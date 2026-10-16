# AI Upscale 2.0.0 verification

Machine-specific JSON reports referenced below remain with local validation runs.
This source repository retains the release summary and the scripts used to run its checks.

Updated October 15, 2026. The plugin now uses package version 2 and the general
`plugin.processImage` API. Its panel sends explicit output dimensions and its own
options. Its worker validates API version, sRGB/RGBA8 input, output dimensions,
models, processing settings and limits. No model ids, inference dependencies or
upscale-specific service are needed in PhotoCraft's core.

Current package: **170,088,981 bytes**. SHA-256:

`187a8d2d094069f26fbf65914fcf44cc0663595e259a0b32a326eb357d2cf4e5`

The current archive is a documentation-only repack clarifying that upgrading retired
version-1 packages can reset panel preferences. Entry-by-entry SHA-256 comparison
confirmed that only `README.md` changed; all 34 other members, including the worker,
panel, models and inference libraries, are identical. CRC, structure, pinned runtime
hashes, PE dependencies and source/build identity were checked again. Inference was
not rerun for this prose-only repack. The 20 packaged processing cases below ran on
prior archive `c42ac92c17d26b0bcf4076b327846eff5a4c4bb0e7aac4c2a25639ab47464894` (170,088,931 bytes) and exercise the identical processing
payload. See `v2.0.0-doc-repack.json` for exact old/new identities and changed-entry hashes.

| Check | Result |
|---|---|
| Independent engine tests | 14 passed, including alpha, tiling, 1× enhancement, cancellation, sizing and long paths |
| Native worker tests | 5 passed; valid generic jobs, same-size output, cancellation/EOF, malformed/oversized jobs, option injection and non-RGBA8 rejection |
| Panel tests | 4 passed; general permission, explicit dimensions, all models, all scales, default, resource limits and reported 4× dimensions |
| Build checks | Native/panel formatting, all-target native/panel Clippy, WASM-target Clippy, release Windows worker and release WASM passed |
| Package checks | CRC, 35 portable noncolliding members, v2 metadata, PE/WASM headers and all 16 pinned runtime asset hashes passed |
| Actual packaged workers | 20 cases passed: seven AI models at 1× and 2×, Lite at 3×/4×, software 1×, a long managed installation path, 24-MP software 1× and reported 192-MP software 4× |
| Output checks | Expected dimensions, source immutability, alpha and embedded sRGB checked in all packaged cases; AI 1× changes RGB detail |
| Dependency closure | PE imports of worker, NCNN engine and bundled VCOMP140 checked; only Windows OS libraries and the GPU driver's Vulkan loader remain external |
| Isolated runtime | All 20 cases ran with PATH containing only Windows and System32, with no developer toolchain directories |

The 4,240 × 2,832 → 16,960 × 11,328 software case completed in 10.675 seconds with
approximately 1.60 GB peak worker working memory/commit on this machine. It confirms
the larger output path, not full-size GPU inference speed or quality. Small synthetic
images exercise model loading and geometry; this is not certification on every GPU
or photograph. Actual host installation, preview, Apply, history and safe-mode
validation are recorded by the PhotoCraft host project separately.

The first test attempt encountered Windows sandbox restrictions in the default
temporary folder. The complete native suite passed using a workspace-local temporary
folder. Source Clippy checks passed; the toolchain reported a cache hard-link fallback
warning unrelated to source code. Raw current results are `package-smoke.json`,
`v2.0.0-release.json`, `v2.0.0-large.json` and the `v2.0.0-*.log` files.

## Earlier 1.3.1 verification (legacy integration)

Updated October 15, 2026. The fresh panel default is now **1× · Enhance only**.
The manifest, guest fallback and processing library agree on the default. Explicit
1×, 2×, 3× and 4× selections remain supported. Valid saved panel selections
are retained by PhotoCraft; this update does not reset preferences.

Earlier package: **170,074,691 bytes**. SHA-256:

`4c47ffc3704f7efbd4777cf9188cce6ea08276df445ff2b2652e6147c9d86046`

Validation: 18 independent engine/worker tests and 3 guest tests passed; formatting,
all-target Clippy with source warnings denied, release worker and release WASM passed.
The actual package passed CRC/path/contract checks and all 16 pinned runtime hashes.
Every bundled model ran at 1× and 2×, plus a software 1× case (15 cases), with
correct size, alpha, sRGB profile and unchanged source. Packaged binaries and manifest
match that source build. Earlier release metadata is in
`v1.3.1-release.json`; command output is in the `v1.3.1-*.log` files.

The initial test run hit sandbox restrictions on Windows' default temporary folder.
The complete suite passed with a workspace-local temporary folder. Model weights,
processing algorithms and the host protocol are unchanged. The integration checks
below describe the previous 1.3.0 model release and were not rerun for this default-only
plugin patch; current host/window checks are recorded separately by the host project.

## Earlier 1.3.0 integration validation

Updated on October 15, 2026. The plugin now includes High Fidelity, Remacri, UltraMix
Balanced and UltraSharp alongside Standard, Fast and Digital Art. The seven choices are
inside PhotoCraft's AI Upscale panel and all weights ship in one installable package.
The host protocol and editor code are unchanged; an existing preview-capable host can
install this update without another editor rebuild.

The package is **170,073,963 bytes (162.2 MiB)**. SHA-256:

`632d63e45ee3231465e0824c90ff18f31c2a2414e6365fd0504058000de3fbd0`

## Install or update

Use the packaged release or build `dist/PhotoCraft-AI-Upscale.pcplugin`:

1. Open **Plugins > Manage Plugins… > Choose plugin…**.
2. Select the package and review its native-code permissions.
3. Select **Replace or repair an existing installation**, then **Install plugin**.
   Older hosts call this replacement option **Replace the installed version**.
4. Open **Plugins > AI Upscale** and choose a model. Generate a preview and compare it
   before Apply. At 1x the result can be applied as an undoable new layer; enlargement
   opens a new result tab.

The plugin keeps its existing id, `org.photocraft.ai-upscale`, for replacement updates.
The original three labels and default model remain compatible with saved panel values.

## Models and terms

| New dropdown choice | Provenance and terms |
|---|---|
| Photo · High Fidelity | Upscayl alias of Philip Hofmann's 4xHFA2k; CC BY 4.0 with attribution |
| Remacri · Non-commercial | Foolhardy / FoolhardyVEVO; CC BY-NC-SA 4.0 |
| UltraMix Balanced · Non-commercial | Kim2091; CC BY-NC-SA 4.0 |
| UltraSharp · Non-commercial | Kim2091; CC BY-NC-SA 4.0 |

The three non-commercial choices are labeled in the dropdown and panel description.
Full license texts, attribution, original source records and unmodified-weight checksums
are included. The source's MIT license does not relicense those weights. High Fidelity's
original training target is anime imagery; Upscayl presents its alias as a general image
option. Its preview should be judged on the particular image. The vendored model metadata
has been corrected from the inaccurate RealESRNet name to HFA2k.

The four additions contribute 134,202,738 bytes (127.99 MiB) of uncompressed model assets.
They increase the plugin package and installed plugin data, not the editor executable.

## Validation

| Check | Result |
|---|---|
| Independent Rust processing/worker tests | 17 passed |
| WASM guest tests | 3 passed; every displayed choice maps to its intended model id |
| Source checks/build | Formatting, all-target Clippy with source warnings denied, release Windows worker and release WASM passed |
| Package structure | CRC, 35 portable noncolliding members, native/WASM headers and all 16 pinned runtime hashes passed |
| Actual packaged AI workers | All seven models passed at 1x and 2x (14 cases), plus software and a long-path 16-bit AI regression |
| Actual installed panel-to-host flow | All four new models passed at 1x and 2x through WASM, host, worker, preview and Apply (8 cases) |
| Pixel/result checks | Correct dimensions, source immutability, transparent/partial/opaque alpha, embedded sRGB, 300 DPI and finite pixels verified |
| Apply/history | Same-size results converted to a 16-bit source layer; Undo/Redo and separate enlargement tabs passed; guest state deferred until Apply |
| Model routing | All four new models produced different decoded visible-RGB fingerprints at each tested scale |
| Shared installed-package lifecycle | Actual software and Vulkan cases passed installation/restart/delete-download/cancellation/stale-source checks |
| UI | Seven-choice dropdown plus longest selected label at 1152x768 and 640x480 visually checked; controls and Done fit |

These cases use small synthetic images and existing engine fixtures. They validate model
loading, routing and integration, not subjective quality or performance on every photograph,
large image or GPU. Existing memory/pixel limits and 8-bit sRGB inference output remain.
No broad editor test suite was rerun for this plugin-only update. Previous editor safety
work and its qualified failures remain documented in `PLUGIN-SAFETY-AUDIT.md`.

Raw results are in `seven-model-*.log/json/png`; standalone worker results are also in
`verification/package-smoke.json`. The independent source
archive is `ai-upscale-plugin-source.zip`.

## Current controls

Model choice, output scale, Detail strength and Extra AI passes retain their existing
behavior. Detail is a blend of the reconstruction with the conventionally resized original.
Extra AI passes use test-time augmentation. These fixed-weight models do not expose
independent denoise, deblur, face recovery or creativity settings. Such controls require
additional processing/model support and are outside this model-bundling update.
