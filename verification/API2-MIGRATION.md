# Panel API 2 migration — 2.1.0

Verified locally on Windows x64 on October 16, 2026. This record identifies the
current API 2 package; earlier validation records describe their own prior artifacts.

- Package: `PhotoCraft-AI-Upscale.pcplugin` (170,152,133 bytes).
- SHA256: `1d982aba0e2e668a5d43d087a1a4039c1430439cc3eaa69ee0cb31cdc2b739e7`.
- Panel API 2; private SDK/API 0.1; package format 2.
- Required capability: `host.commands.invoke` version 1.
- Rust tests: 5 panel and 19 native tests passed.
- Both Cargo workspaces: formatting and all-target Clippy passed.
- Release WASM/native package build, archive CRC, conformance and private SDK hashes passed.
- The panel also passed tests and release WASM compilation from a clean source export,
  without downloaded native runtimes/models or an editor/sibling source dependency.
- Current API-2-only host: install, restart discovery, real processing of a 16-bit source,
  source-preserving preview, layer Apply, Undo/Redo, Discard, new-tab Apply/DPI,
  stale-source rejection, cancellation/reaping and removal passed in a disposable store.

The host acceptance also exercised actual Vulkan 2× enlargement. Existing scale/model defaults and worker image protocol 1 remain intact.

The original API 1 panel was retained as a negative control and rejected by the current host. Its stable plugin id is preserved in the rebuilt API 2 package.

The three migrated panels use the SDK's bounded ABI buffers and typed event/response
contracts. `authoring.json` is the descriptor/package source; generated output is not edited.
Panel versions, authoring format, package versions, host metadata and worker protocols are
separate contracts. Retirement of panel API 1 does not retire worker protocol 1.

These are engine integration checks. Desktop click-through was not repeated. No user
installation was modified during validation. Published packages and source are available in the repository Releases page.
Machine-specific details are in `api2-build-checks.json` and `api2-*.log` beside this file;
workspace acceptance/conformance and clean-source evidence are under `extension-review/`.
