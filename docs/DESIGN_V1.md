# SpeedReplace v1 design baseline and roadmap

Status: historical design baseline and future roadmap
Last updated: 2026-11-02  
Audience: contributors and coding agents

> [!IMPORTANT]
> This document records the original v1 design direction and includes deferred
> ideas. It is not a claim that every item below shipped in v1.0.1. For current
> behavior, use the [README](../README.md), [user guide](USER_GUIDE.md), and
> [release notes](../RELEASE_NOTES.md) as the authoritative sources.

Deferred items include rule presets, CLI support, progress and cancellation,
explicit encoding overrides, backup-retention controls, and hunk-level
selection. They remain roadmap candidates rather than v1.0.1 features.

## 1. Purpose

SpeedReplace is a local desktop utility for inspecting and applying text
replacements across many files. It must preserve the convenience of the old
`speeeeed` application while adding the safeguards expected from a modern
tool.

The defining workflow is:

1. Select files or folders.
2. Define one or more ordered rules.
3. Scan and preview every proposed change.
4. Select the changes to apply.
5. Back up the original files and apply changes safely.
6. Verify the result and retain enough information to undo it.

Safety takes precedence over raw throughput when the two conflict. Performance
still matters: preview must be fast enough that users do not feel encouraged
to bypass it.

## 2. Goals and non-goals

### 2.1 Goals

- Run on supported Windows, macOS, and Linux desktop versions.
- Work without an account, network connection, or remote service.
- Handle thousands of files without blocking the UI.
- Preserve the source encoding, BOM, and dominant line-ending style.
- Refuse lossy output unless the user explicitly changes the output encoding.
- Make the exact proposed modifications inspectable before writing.
- Recover original content after a completed or partially failed operation.
- Keep the replacement engine independent of Tauri and the React UI.

### 2.2 Non-goals for v1.0

- Editing binary formats such as DOCX, PDF, or database files.
- Replacing text inside archives.
- Remote, cloud, FTP, or SSH file access.
- Real-time collaboration or account synchronization.
- AI-generated rules.
- A public extension/plugin API.
- Full filesystem transactions across multiple volumes.
- Automatic modification of files whose encoding cannot be determined safely.

CLI support is an architectural requirement but not a v1.0 release blocker.

## 3. Functional requirements

### 3.1 Targets

- Add individual files and folders using a system picker.
- Accept files and folders by drag and drop.
- Recurse into folders when enabled (default: enabled).
- Deduplicate overlapping targets by canonical path where possible.
- Display inaccessible, missing, and unsupported targets as warnings.
- Do not follow directory symlinks by default.
- Treat file symlinks as links and skip them by default in v1.0.

### 3.2 Filters

- Include and exclude patterns use gitignore-style glob semantics.
- Default directory exclusions: `.git`, `.svn`, `.hg`, `node_modules`,
  `target`, `dist`, `build`, and the SpeedReplace backup directory.
- Users can disable or edit the defaults in advanced mode.
- Hidden files are included unless excluded by a rule.
- Files detected as binary are skipped and reported.
- Maximum file size is configurable; default is 100 MiB.

### 3.3 Replacement rules

Each rule contains:

- stable ID;
- enabled state;
- search text;
- replacement text;
- mode: literal or regular expression;
- case-sensitive flag;
- whole-word flag (literal mode only);
- multiline flag;
- optional description.

Rules execute from top to bottom. The output of one rule is the input of the
next. Reordering rules invalidates the current preview.

An empty search expression is invalid. Invalid regular expressions block the
scan and are shown beside the affected rule. Replacement syntax follows Rust
`regex` capture expansion (`$1` and `${name}`) in regular-expression mode.

Before scanning, the engine warns about obvious rule interactions, including:

- a replacement containing a later rule's search term;
- two identical searches;
- a literal rule whose replacement contains its own search text;
- enabled rules that make no semantic change.

These are warnings, not blanket execution errors.

### 3.4 Scan and preview

A scan never writes user files. It returns:

- number of discovered, scanned, matched, skipped, and failed files;
- total match count by rule and by file;
- source encoding and confidence/reason;
- detected line-ending style;
- before/after hashes;
- a unified or structured line diff;
- warnings and errors.

The UI supports:

- expanding a file to see its diff;
- selecting or deselecting whole files;
- selecting individual hunks where independent application is safe;
- filtering the preview by changed, warning, error, or skipped state;
- searching within result paths;
- canceling a scan.

Any change to targets, filters, encoding policy, or rules marks the preview as
stale. A stale preview cannot be applied.

### 3.5 Apply

Apply accepts a scan ID plus selected file and hunk IDs. Before writing each
file, the engine checks that its current size, modification timestamp, and
content hash still match the scan snapshot. A mismatch is a conflict and is
never overwritten silently.

For each eligible file, the engine:

1. Creates and verifies its backup record.
2. Re-reads and hashes the current file.
3. Recomputes the selected transformation.
4. Encodes the result without replacement characters or data loss.
5. Writes and flushes a temporary file in the destination directory.
6. Preserves supported permissions.
7. Atomically replaces the destination where the platform permits.
8. Reads the written file and verifies its hash.
9. Records success or failure in the operation manifest.

Failures are isolated by file. The operation report clearly distinguishes
changed, conflicted, skipped, and failed files. The user may undo the files
that were changed successfully.

### 3.6 Backup and undo

Backups live outside the target tree in the platform application-data folder.
Each operation has an immutable manifest and content-addressed original files.
The manifest records:

- operation ID and timestamps;
- application and schema versions;
- original absolute path;
- original and resulting hashes;
- encoding, BOM, and line-ending metadata;
- original size, permissions, and modification timestamp;
- backup object location;
- per-file apply result.

Undo first compares the current file hash with the recorded resulting hash.
If they differ, the file is a conflict and requires an explicit choice. The
default choices are skip or restore to a separate path; overwriting a changed
file is an advanced, separately confirmed action.

Retention defaults to the latest 20 operations and 30 days. Automatic cleanup
must never delete the only backup for an operation currently being undone.

### 3.7 History and presets

v1.0 stores local operation history and replacement-rule presets. Presets do
not include target paths unless the user opts in. History may be cleared from
settings without touching user files.

## 4. Encoding and text policy

### 4.1 Initial support

Required for v1.0:

- ASCII and UTF-8, with or without BOM;
- UTF-16 LE and BE, with BOM;
- Windows-31J / CP932 (presented as Shift_JIS/CP932 in the UI);
- EUC-JP;
- ISO-2022-JP;
- Windows-1252.

UTF-32 may be added if the selected Rust libraries support round-trip
validation cleanly; it is not required for the first implementation milestone.

### 4.2 Detection order

1. Recognize a BOM and validate the corresponding encoding.
2. Accept valid UTF-8.
3. Apply deterministic heuristics to the configured legacy encodings.
4. Return `Ambiguous` when no candidate reaches the required confidence.

Ambiguous files are previewable only after a per-file or scan-level encoding
override. They are never written based solely on a low-confidence guess.

### 4.3 Round-trip validation

Decoded text must encode back to the exact original bytes before any edit is
considered safe. The transformed text must also encode without unmappable
characters. A failed round trip produces an error and prevents writing.

### 4.4 Line endings

The decoder records LF, CRLF, CR, or mixed endings. Replacement content entered
with logical newlines is normalized to the file's dominant ending. Existing
untouched line endings remain unchanged. Mixed-ending files show a warning and
are not globally normalized by default.

## 5. User experience

### 5.1 Main window

The main window has four persistent areas:

1. Targets and filters.
2. Replacement rules.
3. Preview/results.
4. Progress, warnings, and primary actions.

Simple mode shows target selection, a single rule, the preview button, and safe
defaults. Advanced mode exposes multiple rules, globs, encoding overrides,
symlink policy, size limits, and retention controls. Switching modes preserves
all values; it only changes visibility.

### 5.2 Primary states

- Empty: invite the user to add a file or folder.
- Ready: valid targets and rules; preview is available.
- Scanning: determinate progress where totals are known; cancel is available.
- Preview: results are inspectable; apply is available if no blocking errors
  exist.
- Stale preview: explain what changed and require a new scan.
- Applying: cancel stops before the next file, never during a destination
  replace.
- Complete: summarize results and offer undo/history.
- Undo preview: list safe restores and conflicts before restoring.

### 5.3 Accessibility and localization

- Full keyboard operation and visible focus indicators.
- Semantic labels for controls and diff status.
- Do not rely on color alone for additions, deletions, warnings, or errors.
- Respect OS light/dark preference, with a manual override.
- Keep UI strings in localization resources from the start.
- Initial locales: English and Japanese.

## 6. Architecture

```text
React UI
  -> typed frontend service
    -> Tauri commands and events
      -> application service (Rust)
        -> speedreplace-core
          -> discovery
          -> decoding/encoding
          -> rule engine
          -> diff model
          -> apply/verification
        -> backup/history repository
        -> platform filesystem adapter
```

The core crate must not depend on Tauri. Filesystem access is behind traits so
unit tests can use temporary or in-memory implementations.

Suggested workspace:

```text
SpeedReplace/
  apps/desktop/              React and Tauri application
  crates/core/               Pure replacement domain and engine
  crates/storage/            Backup, manifest, history, settings
  crates/platform/           Native filesystem behavior
  packages/contracts/        Generated/shared TypeScript types
  docs/
  fixtures/                  Encoding and filesystem test corpus
```

Avoid introducing a database initially. Versioned JSON manifests and settings
are sufficient for v1.0 and are easier to inspect and recover. Reconsider
SQLite only if history querying or backup deduplication becomes a measured
bottleneck.

## 7. Domain model

Core identifiers are UUIDs serialized as strings.

```text
ScanRequest
  targets: TargetSpec[]
  filters: FilterSpec
  rules: ReplacementRule[]
  encodingPolicy: EncodingPolicy
  options: ScanOptions

ScanResult
  id: ScanId
  requestFingerprint: Hash
  files: FilePreview[]
  summary: ScanSummary
  createdAt: Timestamp

FileSnapshot
  path: Path
  size: u64
  modifiedAt: optional Timestamp
  sourceHash: Hash
  encoding: EncodingInfo
  lineEndings: LineEndingInfo

ApplyRequest
  scanId: ScanId
  selectedFiles: FileSelection[]

OperationManifest
  id: OperationId
  scanId: ScanId
  status: Applying | Completed | CompletedWithErrors | Interrupted
  files: OperationFileRecord[]
```

Paths remain native OS paths in Rust. When crossing the Tauri boundary, they
are serialized using an explicit lossless representation; the implementation
must not assume all paths are valid UTF-8.

## 8. Tauri boundary

Initial commands:

- `get_app_state()`
- `validate_rules(rules)`
- `start_scan(request) -> job_id`
- `cancel_job(job_id)`
- `get_scan(scan_id)`
- `start_apply(request) -> job_id`
- `list_operations(page)`
- `get_undo_preview(operation_id)`
- `start_undo(request) -> job_id`
- `load_settings()` / `save_settings(settings)`
- `load_presets()` / `save_preset(preset)` / `delete_preset(id)`

Long work reports events containing `job_id`, monotonically increasing
sequence number, phase, completed count, optional total, and a final result.
The frontend ignores events from obsolete jobs.

The Tauri allowlist grants only the minimum dialog, drag/drop, and filesystem
capabilities needed by these commands. The webview never receives arbitrary
filesystem access.

## 9. Concurrency and performance

- Discovery and scanning run outside the UI thread.
- Cancellation is cooperative and checked between files and bounded chunks.
- Use a bounded worker pool; default concurrency is based on available CPUs but
  capped to avoid saturating storage.
- Preserve deterministic preview order by sorting normalized display paths.
- Do not load all file contents and all rendered diffs into the webview.
- Large previews are paged or virtualized.
- Apply writes are deliberately conservative; parallel writes are disabled for
  the first implementation and enabled only after correctness tests.

Initial performance targets on an SSD reference machine:

- UI remains responsive during every operation.
- Scan 10,000 small UTF-8 files (1 GiB total) in under 10 seconds after warmup.
- Display the first preview results within 500 ms when matches are available.
- Keep steady-state application memory below 300 MiB for that fixture.

Targets are benchmarks, not correctness criteria. The benchmark machine and
fixture must be recorded with every reported result.

## 10. Error handling and logging

Errors have stable machine codes, a user-safe message, path context when
relevant, and an optional technical cause for diagnostics. Expected conditions
such as permissions, ambiguous encoding, and external modification are normal
per-file results rather than application crashes.

Logs are local, rotating, and contain no file contents, search strings, or
replacement strings by default. Paths are included only at a user-selected
diagnostic level. Crash reports are never uploaded automatically.

## 11. Security and privacy

- No telemetry or network permissions in the default build.
- Validate all paths again in Rust; never trust webview payloads.
- Prevent backups from escaping the application-data root.
- Use unpredictable temporary names and create-new semantics.
- Never execute target content or replacement text.
- Bound regex work where practical and allow cancellation of expensive scans.
- Treat operation manifests as data, not commands.
- Package and updater signing are required before public distribution.

## 12. Testing strategy

### 12.1 Core unit tests

- Literal, regex, multiline, case, whole-word, and capture replacements.
- Ordered-rule interaction and deterministic results.
- Empty and invalid rules.
- Line-ending detection and preservation.
- Encoding detection and exact round trips.
- Unmappable output rejection.

### 12.2 Property and fuzz tests

- Decode/encode round-trip fixtures never change untouched bytes.
- Applying an empty selection never changes files.
- Previewed full-file output equals applied output from the same snapshot.
- Manifest parsing never escapes the backup root.
- Fuzz regex replacement parsing, encoding inputs, and manifests.

### 12.3 Integration tests

- Nested discovery, exclusions, hidden files, and symlink policy.
- Permission errors and disappearing files.
- External modification between scan and apply.
- Partial failure followed by undo.
- Process interruption leaves the original or verified replacement, never a
  truncated destination.
- Windows reserved names and long paths; macOS normalization; Linux non-UTF-8
  paths where supported.

### 12.4 UI and release tests

- Simple-mode happy path and advanced multi-rule flow.
- Keyboard-only operation and automated accessibility checks.
- Stale-preview prevention.
- Installer launch and a signed-package smoke test on all three OS families.

The fixture corpus must contain every supported encoding, BOM combination,
line-ending style, mixed endings, invalid byte sequences, large files, and
representative Japanese text.

## 13. Build and release

CI should run formatting, linting, Rust tests, frontend tests, dependency
auditing, and builds on Windows, macOS, and Linux. Public release artifacts are:

- Windows: MSI and/or NSIS installer;
- macOS: signed and notarized DMG;
- Linux: AppImage and Debian package initially.

Releases require reproducible version metadata, checksums, changelog, software
bill of materials, and signed artifacts where supported. Automatic updating is
deferred until signing and rollback behavior are designed and tested.

## 14. Implementation milestones

### M0: repository foundation

- Decide the license and supported toolchain versions.
- Create the Rust workspace, Tauri/React app, linting, formatting, and CI.
- Establish generated Rust/TypeScript contracts.

### M1: core UTF-8 preview

- File discovery and filtering.
- Literal replacement and structured diff.
- Read-only scan command and simple-mode UI.

### M2: rule engine

- Regex, multiline, whole-word, and ordered multiple rules.
- Validation and rule-interaction warnings.
- Advanced-mode rule editor.

### M3: encoding safety

- Required encodings, BOM and line-ending preservation.
- Confidence display, overrides, and round-trip rejection.
- Complete encoding fixture corpus.

### M4: safe apply

- Snapshot conflict checks, backup manifest, temporary write, atomic replace,
  and post-write verification.
- Apply report and interrupted-operation recovery.

### M5: undo and product completeness

- Conflict-aware undo, history, retention, presets, drag/drop, localization,
  accessibility, and virtualized results.

### M6: cross-platform release

- Performance work based on benchmarks.
- OS-specific integration tests, signing/notarization, installers, and release
  documentation.

Each milestone must keep the test suite green and must not introduce a write
path before its related recovery path and tests exist.

## 15. Acceptance criteria for v1.0

v1.0 is ready only when:

- all required replacement and encoding modes pass the fixture suite;
- unchanged bytes outside selected replacements are preserved;
- stale previews and externally changed files cannot be silently applied;
- every successful modification has a verified backup and manifest record;
- undo handles both unchanged and conflicted destinations safely;
- cancellation and simulated interruption cannot produce truncated originals;
- the primary workflow is keyboard accessible in English and Japanese;
- signed release candidates pass smoke tests on Windows, macOS, and Linux;
- no default runtime network connection or telemetry is present.

## 16. Decisions still required

These are intentionally not resolved by this document:

1. Public name clearance for SpeedReplace (software trademarks, GitHub names,
   package names, and domains).
2. MIT versus Apache-2.0, or dual licensing.
3. Minimum supported OS versions.
4. Exact signing identities and release channels.
5. Whether CLI ships in v1.0 or immediately afterward.
6. Whether UTF-32 is promoted into the required v1.0 encoding set.

None of these blocks M1 core prototyping, but items 1-4 must be settled before
public distribution.
