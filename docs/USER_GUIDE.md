# SpeedReplace v1.1.0 development user guide

The interface starts in English. Use the language selector in the application
header to switch between English and Japanese. The selection is retained on
the device.

## Safe replacement workflow

1. Add files or folders, or drop them onto the window.
2. Enter the search and replacement text.
3. Select **Search & Preview** (`検索・プレビュー`).
4. Inspect the changed-file count and expand files to review individual diff
   hunks.
5. Deselect any file or individual hunk that should remain unchanged.
6. Select **Replace**. SpeedReplace verifies and stores every original before
   changing the destination.

If a file changes after preview, SpeedReplace reports a conflict and does not
overwrite it.

## Advanced mode

Advanced mode adds:

- multiple ordered and individually enabled rules;
- literal or regular-expression mode;
- case-sensitive and whole-word matching;
- gitignore-style include and exclude patterns;
- recursive discovery and a maximum file-size limit.
- reusable rule presets;
- automatic detection or a manual default/per-file encoding override;
- the number of backup operations to retain.

Regular-expression replacements accept Rust `regex` capture references such
as `$1` and `${name}`. Rules run from top to bottom, and each rule receives the
output of the previous rule.

## Encodings

SpeedReplace detects and preserves:

- UTF-8, including BOM;
- UTF-16 LE/BE with BOM;
- Shift_JIS/CP932;
- EUC-JP;
- ISO-2022-JP when detected by the encoding engine;
- Windows-1252.

Every decoded file must pass an exact round trip before it is considered safe.
Replacement text that cannot be represented in the original encoding is not
written. LF, CRLF, CR, and mixed line endings are reported and preserved.
When automatic detection is ambiguous, Advanced mode can force a supported
encoding globally or for one file. A changed override invalidates the current
preview and requires another scan.

## Progress and cancellation

Scanning and applying display completed and total file counts plus the current
path. **Cancel** stops before the next file. During apply, files already
completed remain safely changed with backup manifests; the history can undo
them. A file is never interrupted midway through its atomic replacement.

## Rule presets and backup retention

Advanced mode can save the current ordered rule list as a local preset and
load or delete it later. Presets remain on the device. The backup retention
setting keeps the newest configured number of operation directories after a
successful apply.

## Command-line interface

The `speedreplace` binary provides `scan`, `replace`, `history`, and `undo`
commands. `scan` never writes target files. `replace` uses the same stale-file
checks, verified backups, encoding preservation, and atomic writes as the
desktop app. Use `--rules` for ordered JSON rules and `--json` for structured
automation output.

## Undo

Open **History & Undo** (`履歴・Undo`) and choose an operation. SpeedReplace
checks every current file against the hash recorded immediately after the
replacement. Files that have not changed can be restored. Externally modified
files are reported as conflicts and skipped.

Backups and manifests are stored in the operating system's application-data
directory, outside the target tree.

## Symbolic links

SpeedReplace skips symbolic-link targets and does not follow directory links.
If a file is replaced with a symbolic link after preview, applying or undoing
the operation reports a conflict instead of modifying the link or its target.

