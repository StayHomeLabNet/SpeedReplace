# SpeedReplace

[English](README.md) | [日本語](README.ja.md)

SpeedReplace is a cross-platform desktop application for fast, safe batch text
replacement.

The project combines the straightforward workflow of `speeeeed` with modern
file-format handling and safeguards against accidental data loss.

## Product principles

1. Simple enough for a one-off replacement.
2. Safe enough for a large directory tree.
3. Fast enough that preview remains the normal workflow.
4. Local and offline by default.

## Technology stack

- Tauri 2
- Rust replacement engine
- React and TypeScript user interface
- Windows, macOS, and Linux

## v1.0 features

- Files and recursive folder targets
- Literal, regular-expression, and multiline replacement
- Ordered multiple replacement rules
- File include/exclude filters and binary-file detection
- UTF-8, UTF-16, and Japanese legacy-encoding support
- Encoding, BOM, and line-ending preservation
- Per-file diff preview and dry run
- Automatic backup and conflict-aware undo
- Drag and drop
- Simple and advanced modes

The implementation specification is in
[docs/DESIGN_V1.md](docs/DESIGN_V1.md).

## Project status

SpeedReplace v1.0 is implemented. It includes the independent Rust engine,
Tauri command layer, React desktop interface, verified backup/apply/undo flow,
and a cross-platform CI definition. See the [user guide](docs/USER_GUIDE.md)
for the workflow and safety behavior.

## Development

Requirements: Node.js 24+, pnpm 11+, the stable Rust toolchain, and the native
Tauri prerequisites for the host operating system.

```bash
pnpm install
cargo test --workspace
pnpm --dir apps/desktop build
pnpm --dir apps/desktop tauri dev
```

Create a local production bundle with:

```bash
pnpm --dir apps/desktop tauri build
```

Windows and Linux distribution packages are built by the **Package desktop
apps** GitHub Actions workflow. Run it manually, or push a `v*` tag, then
download:

- `SpeedReplace-Windows-x64`: MSI and NSIS (`.exe`) installers
- `SpeedReplace-Linux-x64`: Debian (`.deb`) and AppImage packages

Workflow artifacts are retained for 30 days.

The software is released under the MIT License. Public binaries still require
platform signing/notarization credentials owned by the distributor.

