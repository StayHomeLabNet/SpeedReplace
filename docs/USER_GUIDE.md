# SpeedReplace v1.0 user guide

## Safe replacement workflow

1. Add files or folders, or drop them onto the window.
2. Enter the search and replacement text.
3. Select **Search & Preview** (`検索・プレビュー`).
4. Inspect the changed-file count and expand files to review their unified
   diffs.
5. Deselect any file that should remain unchanged.
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

## Undo

Open **History & Undo** (`履歴・Undo`) and choose an operation. SpeedReplace
checks every current file against the hash recorded immediately after the
replacement. Files that have not changed can be restored. Externally modified
files are reported as conflicts and skipped.

Backups and manifests are stored in the operating system's application-data
directory, outside the target tree.

