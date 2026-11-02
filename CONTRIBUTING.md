# Contributing to SpeedReplace

Thank you for helping improve SpeedReplace.

## Before opening an issue

- Search existing issues and release notes first.
- Use a minimal example that contains no private or confidential files.
- For security issues, follow [SECURITY.md](SECURITY.md) instead of opening a
  public issue.

## Development setup

Install Node.js 24 or later, pnpm 11, the stable Rust toolchain, and the native
Tauri prerequisites for your operating system.

```bash
pnpm install --frozen-lockfile
cargo test --workspace
pnpm --dir apps/desktop build
pnpm --dir apps/desktop tauri dev
```

## Pull requests

1. Keep each pull request focused on one change.
2. Add or update tests for behavior changes.
3. Preserve file encodings, BOMs, line endings, backups, and conflict detection.
4. Run the complete local validation suite:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir apps/desktop build
```

5. Update English and Japanese UI strings together. See
   [docs/LOCALIZATION.md](docs/LOCALIZATION.md).

By contributing, you agree that your contribution is licensed under the MIT
License used by this repository.
