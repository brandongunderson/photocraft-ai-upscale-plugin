# PhotoCraft Plugin API compatibility

## Release 2.1.1 — 2026-10-16

Validated locally on Windows x64 against the current PhotoCraft Plugin API host.
The plugin privately includes its SDK/API development sources. It builds without the editor
source and requires no sibling plugin or separately installed SDK runtime. Package format 2
and its declared native protocol remain independent.

- Package: `PhotoCraft-AI-Upscale.pcplugin` (170,152,122 bytes).
- SHA-256: `5168d4a52439124588581270d0845cebaaf4245686004a5a50d9fcdbe3791eee`.
- Final update-metadata revision: 5 panel tests, formatting and all-target Clippy passed.
  The unchanged native worker previously passed 19 tests, formatting and all-target Clippy.
- Release WASM/native builds and package conformance passed. Archive CRCs, all
  35 private SDK files, 16 pinned runtime/model assets, packaged README
  and exact compiled panel/worker bytes were verified.
- The embedded descriptor declares this plugin's [GitHub repository](https://github.com/brandongunderson/photocraft-ai-upscale-plugin)
  and exact update asset `PhotoCraft-AI-Upscale.pcplugin`. Conformance verified both fields;
  update checks and installation require explicit user actions.
- Real-host acceptance on the final package passed: reviewed installation, restart discovery,
  refusal of an undeclared public action, native processing, source-preserving preview,
  layer Apply, Undo/Redo, Discard, new-tab Apply with DPI preserved, stale-source refusal,
  cancellation/reaping preserving source and panel state, and removal.
- Local Vulkan AI inference produced a 64×48 PNG from a 32×24 source using Photo · Fast at 2×.

## Scope and limits

The integration run used a deterministic 32×24 synthetic image in a 16-bit document.
Native image transport and previews are 8-bit sRGB; layer Apply preserved the document's
16-bit depth. Removal plugins exercised their default model/output. This run does not
qualify every model, large-image performance or additional operating systems. Desktop
click-through and installation of a future GitHub release were outside this run. User
installations and normal plugin-data folders were untouched; all host stores and plugin
user data were isolated.

Host verification used Rust 1.99.0 (`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`),
`x86_64-pc-windows-gnu`. Machine-specific logs, PNGs and host-library fingerprints remain
local verification artifacts. See [README](../README.md) for processing, platform and license limits.
