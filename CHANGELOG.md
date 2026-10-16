# AI Upscale changelog

Release history for this standalone PhotoCraft plugin. Versions below refer to this plugin;
the PhotoCraft Plugin API and package/worker formats have separate compatibility markers.

## [2.1.1](https://github.com/brandongunderson/photocraft-ai-upscale-plugin/releases/tag/v2.1.1) - 2026-10-16

### Added

- Opt into PhotoCraft’s explicit GitHub release checks using this plugin’s own repository and
  exact package asset. Updates require review and confirmation.

### Changed

- Refresh the privately bundled PhotoCraft Plugin API and typed SDK sources.
- Preserve the stable plugin identity, controls, model choices and native processing protocols.
- Update independent-build and compatibility documentation.

### Validation

The final descriptor passed 5 panel tests, formatting and all-target Clippy. The unchanged
native worker previously passed 19 tests and its formatting/Clippy checks. Final release
packaging, exact GitHub update metadata and current-host conformance passed. Real native
processing, preview, Apply/Discard, Undo/Redo, stale-source rejection, cancellation and removal
passed on the final package in isolated stores. See [API compatibility](verification/API-COMPATIBILITY.md)
for the package hash and limits.

## [2.1.0](https://github.com/brandongunderson/photocraft-ai-upscale-plugin/releases/tag/v2.1.0) - 2026-10-16

### Changed

- Use privately included typed SDK/API sources and bounded request/response storage.
- Generate the embedded descriptor, bindings and package metadata from `authoring.json`.
- Declare the required `host.commands.invoke` capability and preserve the stable plugin ID,
  saved settings and model choices. Observer and unknown events preserve state without
  starting processing.

### Processing retained in this release

- Seven bundled local AI models, 1x enhancement and 2x/3x/4x enlargement, plus an explicit
  conventional Lanczos option.
- Tiled inference, transparency handling, progress/cancellation and the existing native image
  protocol. Preview, Apply and Undo use PhotoCraft's shared image-processing interface.
- A complete Windows x64 package with its inference engine, runtime dependencies, model
  provenance and license notices. AI inference requires a compatible Vulkan GPU/driver.

### Compatibility

Uses package format 2 and
`host.commands.invoke` version 1. The native image protocol remains version 1. The plugin builds without PhotoCraft source or another plugin.
Remacri, UltraMix Balanced and UltraSharp retain their non-commercial model terms.

See [API compatibility](verification/API-COMPATIBILITY.md) for the release checks and
acceptance limits, and [README.md](README.md) for installation and processing limits.
