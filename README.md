# SpeedReplace

SpeedReplace is a planned cross-platform desktop application for fast, safe,
batch text replacement.

The project combines the straightforward workflow of `speeeeed` with modern
file-format handling and safeguards against accidental data loss.

## Product principles

1. Simple enough for a one-off replacement.
2. Safe enough for a large directory tree.
3. Fast enough that preview remains the normal workflow.
4. Local and offline by default.

## Planned stack

- Tauri 2
- Rust replacement engine
- React and TypeScript user interface
- Windows, macOS, and Linux

## v1.0 scope

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

The v1.0 design baseline has been created. Implementation has not started yet.
The public product name, software license, signing identities, and release
channels still need final confirmation.

