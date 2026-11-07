# ⚡ SpeedReplace

## Fast, safe cross-platform batch text replacement

**SpeedReplace previews changes before applying them, preserves text encoding and
line endings, and creates verified backups for safe Undo.**

[English](README.md) | [日本語](README.ja.md)

[Features](#features) ・ [Supported file formats](#supported-file-formats) ・ [Release](#release) ・ [Development](#development) ・ [Localization](#localization) ・ [Contributing and security](#contributing-and-security)

[![CI](https://github.com/StayHomeLabNet/SpeedReplace/actions/workflows/ci.yml/badge.svg)](https://github.com/StayHomeLabNet/SpeedReplace/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/StayHomeLabNet/SpeedReplace)](https://github.com/StayHomeLabNet/SpeedReplace/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-orange)
![App](https://img.shields.io/badge/app-Tauri%202-purple)
![Files](https://img.shields.io/badge/files-plain%20text-brightgreen)
![Encoding](https://img.shields.io/badge/encoding-UTF--8%20%7C%20UTF--16%20%7C%20Japanese%20legacy-lightgrey)

[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20StayHomeLabNet-ff5e5b?style=for-the-badge&logo=kofi&logoColor=white)](https://ko-fi.com/stayhomelabnet)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Support%20StayHomeLabNet-ffdd00?style=for-the-badge&logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/stayhomelabnet)

**If SpeedReplace is useful to you, please consider supporting the project.**

---

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

## Features

- Files and recursive folder targets
- Literal, regular-expression, and multiline replacement
- Ordered multiple replacement rules
- File include/exclude filters and binary-file detection
- UTF-8, UTF-16, and Japanese legacy-encoding support
- Encoding, BOM, and line-ending preservation
- Per-file diff preview and dry run
- Per-hunk selection inside each file preview
- Live scan/apply progress with safe cancellation between files
- Crash-recoverable operation journals with partial-operation Undo
- Reusable rule presets
- Automatic backup retention limits
- Automatic or manually overridden text encoding
- Automatic backup and conflict-aware undo
- A standalone CLI using the same safe replacement engine
- Drag and drop
- Simple and advanced modes

## Supported file formats

SpeedReplace works with plain-text files. Support is determined by file content
and encoding, not only by the filename extension. Common examples include:

- Text and documents: `.txt`, `.csv`, `.tsv`, `.log`, `.md`, `.rst`, `.adoc`,
  `.tex`
- Structured data and markup: `.json`, `.jsonl`, `.xml`, `.html`, `.htm`,
  `.yaml`, `.yml`, `.toml`, `.sql`, `.graphql`, `.gql`
- Web and templates: `.php`, `.phtml`, `.php5`, `.css`, `.scss`, `.sass`,
  `.less`, `.vue`, `.svelte`, `.astro`, `.twig`, `.blade.php`, `.erb`
- Configuration and build files: `.ini`, `.cfg`, `.conf`, `.properties`,
  `.env`, `.editorconfig`, `.gitignore`, `.gradle`, `.cmake`, `.mk`, plus
  extensionless files such as `Dockerfile` and `Makefile`
- Source code and scripts: `.rs`, `.js`, `.ts`, `.tsx`, `.jsx`, `.py`, `.rb`,
  `.java`, `.c`, `.cpp`, `.h`, `.cs`, `.go`, `.swift`, `.kt`, `.kts`,
  `.scala`, `.lua`, `.pl`, `.pm`, `.r`, `.dart`, `.ex`, `.exs`, `.erl`,
  `.hrl`, `.fs`, `.fsx`, `.vb`, `.sh`, `.ps1`, `.bat`, `.cmd`

These are examples, not an extension whitelist. Other extensionless or
custom-extension files are supported when their contents use a supported text
encoding and pass validation.

Supported encodings are UTF-8, UTF-16 LE/BE, Shift_JIS, EUC-JP, ISO-2022-JP,
and Windows-1252. UTF-8 and UTF-16 BOMs are preserved. Files must pass exact
decoding and round-trip validation before replacement.

Binary and container formats such as `.docx`, `.xlsx`, `.pdf`, archives,
databases, images, audio, and video are not supported. Unsupported files are
hidden from the preview by default and can be reviewed with **Show unsupported
files**. Structured formats are processed as text; SpeedReplace does not parse
CSV, JSON, XML, or source-code syntax.

The original design baseline and roadmap are in
[docs/DESIGN_V1.md](docs/DESIGN_V1.md).

## Project status

SpeedReplace v1.1.0 is the current stable release. It includes the independent Rust engine,
Tauri command layer, React desktop interface, verified backup/apply/undo flow,
and a cross-platform CI definition. See the [user guide](docs/USER_GUIDE.md)
for the workflow and safety behavior.

## Release

The current stable release is **v1.1.0**. See the bilingual
[release notes](RELEASE_NOTES.md) for features, supported packages, validation,
and distribution notes. Download installers from the
[v1.1.0 release](https://github.com/StayHomeLabNet/SpeedReplace/releases/tag/v1.1.0)
and verify them with the attached `SHA256SUMS.txt`.

## Development

Requirements: Node.js 24+, pnpm 11+, the stable Rust toolchain, and the native
Tauri prerequisites for the host operating system.

```bash
pnpm install
cargo test --workspace
pnpm --dir apps/desktop build
pnpm --dir apps/desktop tauri dev
```

Build and use the command-line interface with:

```bash
cargo build --release -p speedreplace-cli
speedreplace scan ./src --search old --replacement new
speedreplace replace ./src --search old --replacement new --retain 20
speedreplace history --backup-dir .speedreplace
speedreplace undo <operation-id> --backup-dir .speedreplace
```

`scan` is read-only. `replace` creates verified backups first. Add `--regex`,
`--include`, `--exclude`, `--encoding`, or `--rules rules.json` as needed; run
`speedreplace <command> --help` for the complete options.

Create a local production bundle with:

```bash
pnpm --dir apps/desktop tauri build
```

Windows, Linux, and macOS distribution packages are built by the **Package
desktop apps** GitHub Actions workflow. Run it manually, or push a `v*` tag,
then download:

- `SpeedReplace-Windows-x64`: MSI and NSIS (`.exe`) installers
- `SpeedReplace-Linux-x64`: Debian (`.deb`) and AppImage packages
- `SpeedReplace-macOS-Universal`: Universal DMG for Apple Silicon and Intel
  Macs

Each workflow artifact also contains the platform-native `speedreplace` CLI
(`speedreplace.exe` on Windows); the macOS CLI is Universal.

Workflow artifacts are retained for 30 days.

## Localization

English is the default UI language, and users can switch languages from the
application header. New languages are added as automatically discovered,
type-checked locale files. See [docs/LOCALIZATION.md](docs/LOCALIZATION.md).

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Report
suspected vulnerabilities privately according to [SECURITY.md](SECURITY.md).

The software is released under the MIT License. Public binaries still require
platform signing/notarization credentials owned by the distributor.

