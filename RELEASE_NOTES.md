# SpeedReplace v1.1.1 (Unreleased)

## English

### Changed

- Persist each file as pending before replacement, then finalize it only after
  verified backup, atomic replacement, and post-write verification succeed.
- Record operation-level running, completed, cancelled, and interrupted states
  with processed and total file counts.
- Reconcile unfinished operations at the next application start and retain the
  changed-file set for safe Undo after a forced exit.
- Show operation status, progress, and per-file results in History & Undo.
- Report how many files were changed when an apply operation is cancelled.

### Validation

- All 24 Rust tests passed.
- Added coverage for cancellation after a partial apply, reopening history, and
  undoing only the files that were changed.
- Added simulated forced-exit recovery coverage from a pending journal record
  through restart reconciliation and Undo.

---

## 日本語

### 変更内容

- 各ファイルを置換前に保留中として永続化し、検証済みバックアップ、原子的置換、
  書き込み後検証が成功した後にのみ変更済みとして確定するように変更
- 操作全体に処理中、完了、キャンセル済み、中断の状態と処理済み・総ファイル数を記録
- 未完了の操作を次回起動時に照合し、強制終了後も変更済みファイルだけを安全にUndo可能
- 履歴・Undo画面に操作状態、進捗、ファイル単位の結果を表示
- 適用のキャンセル時に、それまでに変更されたファイル数を表示

### 検証結果

- Rustテスト24件がすべて合格
- 一部適用後のキャンセル、履歴の再読み込み、変更済みファイルだけのUndoをテスト
- 保留中のジャーナル記録から、強制終了を模した起動時復旧とUndoまでをテスト

---

# SpeedReplace v1.1.0

Release date: November 5, 2026

## English

### Added

- Live discovery, scan, and apply progress with cancellation between files.
- Local rule presets in Advanced mode.
- Configurable retention for verified backup operations.
- Global and per-file manual encoding overrides with strict round-trip checks.
- Selectable diff hunks so only chosen change blocks are applied.
- A standalone `speedreplace` CLI with `scan`, `replace`, `history`, and
  `undo` commands, JSON output, JSON rule files, filters, and encoding options.
- Windows, Linux, and Universal macOS CLI artifacts in the packaging workflow.

### Validation

- All 21 Rust tests passed, including partial-hunk apply, cancellation, manual
  encoding, and backup-retention coverage.
- Rust formatting and Clippy static analysis passed.
- TypeScript checking and the Vite production build passed.
- CLI help and read-only JSON scan smoke tests passed.

### Distribution packages

- Windows x64: MSI, NSIS (`.exe`), and `speedreplace.exe` CLI
- Linux x64: Debian (`.deb`), AppImage, and `speedreplace` CLI
- macOS Universal: DMG and Universal `speedreplace` CLI for Apple Silicon and
  Intel Macs

### Distribution note

Packages are unsigned and may trigger an operating-system security warning.
Verify downloads with the attached `SHA256SUMS.txt`.

---

## 日本語

### 追加内容

- ファイル検出・スキャン・適用の進捗表示と、ファイル間での安全なキャンセル
- 詳細モードで利用できるローカルのルールプリセット
- 検証済みバックアップ操作の保持世代数設定
- 厳密な往復変換検証を行う全体・ファイル単位の文字コード手動指定
- 選択した変更ブロックだけを適用できるハンク単位選択
- `scan`、`replace`、`history`、`undo`、JSON出力、JSONルール、フィルター、文字コード指定に対応した`speedreplace` CLI
- Windows、Linux、Universal macOS用CLIを配布Artifactへ追加

### 検証結果

- ハンク部分適用、キャンセル、文字コード指定、バックアップ保持を含むRustテスト21件がすべて合格
- Rustフォーマット検査とClippy静的解析が合格
- TypeScript型検査とVite製品ビルドが合格
- CLIヘルプと読み取り専用JSONスキャンのスモークテストが合格

### 配布パッケージ

- Windows x64：MSI、NSIS（`.exe`）、`speedreplace.exe` CLI
- Linux x64：Debian（`.deb`）、AppImage、`speedreplace` CLI
- macOS Universal：Apple SiliconとIntel Mac対応のDMGおよびUniversal `speedreplace` CLI

### 配布時の注意

パッケージは未署名のため、OSのセキュリティ警告が表示される場合があります。添付の`SHA256SUMS.txt`でダウンロードを検証してください。

---

# SpeedReplace v1.0.2

Release date: November 3, 2026

## English

SpeedReplace v1.0.2 fixes the packaged UI and completes cross-platform
distribution.

### Changes

- Hide unsupported files from the Preview and apply list by default.
- Add an always-visible **Show unsupported files** checkbox, with the current
  unsupported-file count, on the right side of the step 3 heading.
- Add a clear empty state when a scan contains no supported files.
- Document PHP and other supported plain-text file types while clarifying that
  file support is content- and encoding-based rather than an extension
  whitelist.
- Add automated Universal macOS DMG packaging for Apple Silicon and Intel Macs.

### Validation

- All 16 Rust tests passed.
- Rust formatting and Clippy static analysis passed.
- TypeScript and Vite production builds passed.
- Windows, Linux, and macOS Universal distribution-package builds passed.

### Distribution packages

- Windows x64: MSI and NSIS (`.exe`) installers
- Linux x64: Debian (`.deb`) and AppImage packages
- macOS Universal: DMG for Apple Silicon and Intel Macs

### Distribution note

Packages remain unsigned and may trigger an operating-system security warning.
Verify downloads with the `SHA256SUMS.txt` attached to the GitHub Release.

---

## 日本語

SpeedReplace v1.0.2は、配布バイナリのUIを修正し、クロスプラットフォーム配布を完成させるアップデートです。

### 変更内容

- 非対応ファイルを「プレビューと実行」一覧からデフォルトで非表示に変更
- ステップ3の見出し右側に、非対応ファイル件数付きの **非対応ファイルを表示** チェックボックスを常時表示
- 対応ファイルがないスキャン結果に専用の案内を追加
- PHPなどの対応プレーンテキスト形式を追記し、拡張子の許可リストではなく内容と文字コードで判定することを明記
- Apple SiliconとIntel Macに対応するUniversal DMGの自動パッケージ化を追加

### 検証結果

- Rustテスト16件がすべて合格
- Rustフォーマット検査とClippy静的解析が合格
- TypeScriptおよびVite製品ビルドが合格
- Windows、Linux、macOS Universalの配布パッケージ生成が合格

### 配布パッケージ

- Windows x64：MSIおよびNSIS（`.exe`）インストーラー
- Linux x64：Debian（`.deb`）およびAppImageパッケージ
- macOS Universal：Apple SiliconとIntel Macに対応するDMG

### 配布時の注意

パッケージは引き続き未署名のため、OSのセキュリティ警告が表示される場合があります。GitHub Releaseに添付された`SHA256SUMS.txt`でダウンロードを検証できます。

---

# SpeedReplace v1.0.1

Release date: November 2, 2026

## English

SpeedReplace v1.0.1 is a public-readiness and safety update.

### Changes

- Reject symbolic-link targets during discovery and immediately before writing,
  preventing link replacement and time-of-check/time-of-use surprises.
- Require canonical UUID operation identifiers and validate Undo manifest
  identity and schema.
- Resolve backups from the application-controlled object directory instead of
  trusting a stored backup path.
- Add four safety regression tests, bringing the Rust test suite to 16 tests.
- Replace the Tauri template artwork with a dedicated SpeedReplace icon for all
  desktop platforms.
- Pin GitHub Actions to immutable commit SHAs and update them to current
  Node.js 24-based releases.
- Add automated npm and RustSec audits, Dependabot configuration, a security
  policy, contribution guidance, repository metadata, and release checksums.
- Update the `crc32fast` transitive dependency from 1.5.3 to 1.5.4.

### Validation

- All 16 Rust tests passed.
- Rust formatting and Clippy static analysis passed.
- npm reported no known production dependency vulnerabilities.
- RustSec reported no known vulnerabilities. Two warnings remain in Tauri's
  Linux GTK3 dependency chain: RUSTSEC-2024-0370 and RUSTSEC-2024-0429. The
  application does not directly call the affected APIs; these remain monitored
  until the upstream GUI stack migrates.
- TypeScript, Vite, and Tauri production builds passed.

### Distribution packages

- Windows x64: MSI and NSIS (`.exe`) installers
- Linux x64: Debian (`.deb`) and AppImage packages
- macOS Universal: DMG for Apple Silicon and Intel Macs

### Distribution note

Packages remain unsigned and may trigger an operating-system security warning.
Verify downloads with the `SHA256SUMS.txt` attached to the GitHub Release.

---

## 日本語

SpeedReplace v1.0.1は、公開準備と安全性強化のためのアップデートです。

### 変更内容

- 検索時と書き込み直前にシンボリックリンクを拒否し、リンクの置き換えやTOCTOU問題を防止
- 操作IDを正規形のUUIDに限定し、UndoマニフェストのIDとスキーマを検証
- 保存されたパスを信頼せず、アプリが管理するオブジェクトディレクトリからバックアップを解決
- 安全性の回帰テスト4件を追加し、Rustテストを16件に拡充
- Tauriの初期テンプレート画像を、各デスクトップOS向けのSpeedReplace専用アイコンに変更
- GitHub Actionsを不変のコミットSHAで固定し、Node.js 24対応の現行版へ更新
- npm・RustSec監査、Dependabot、セキュリティポリシー、コントリビューションガイド、リポジトリ情報、リリースチェックサムを追加
- 推移的依存関係`crc32fast`を1.5.3から1.5.4へ更新

### 検証結果

- Rustテスト16件がすべて合格
- Rustフォーマット検査とClippy静的解析が合格
- npmの既知の本番依存脆弱性は0件
- RustSecの既知脆弱性は0件。TauriのLinux GTK3依存連鎖にRUSTSEC-2024-0370とRUSTSEC-2024-0429の警告が残りますが、アプリは対象APIを直接使用しません。上流GUIスタックの移行まで監視を継続
- TypeScript、Vite、Tauriの製品ビルドが合格

### 配布パッケージ

- Windows x64：MSIおよびNSIS（`.exe`）インストーラー
- Linux x64：Debian（`.deb`）およびAppImageパッケージ
- macOS Universal：Apple SiliconとIntel Macに対応するDMG

### 配布時の注意

パッケージは引き続き未署名のため、OSのセキュリティ警告が表示される場合があります。GitHub Releaseに添付された`SHA256SUMS.txt`でダウンロードを検証できます。

---

# SpeedReplace v1.0.0

Release date: November 2, 2026

## English

SpeedReplace v1.0.0 is the first stable release of the fast, safe,
cross-platform batch text replacement application.

### Highlights

- Replace text across individual files or recursive folder trees.
- Use literal, regular-expression, and multiline replacement rules.
- Apply multiple enabled rules in a defined order.
- Preview unified diffs and select files before making changes.
- Preserve UTF-8, UTF-16, Shift_JIS, BOM, and line-ending formats.
- Filter included and excluded paths and skip binary files automatically.
- Create verified backups before writing and safely undo operations later.
- Detect stale previews and external file changes to prevent accidental data loss.
- Switch between Simple and Advanced modes.
- Use the UI in English or Japanese. English is the default, and additional
  languages can be added through type-checked locale files.

### Distribution packages

- Windows x64: MSI and NSIS (`.exe`) installers
- Linux x64: Debian (`.deb`) and AppImage packages
- macOS Apple Silicon: application bundle and DMG can be built from source

### Validation

- 12 Rust engine and safety tests passed.
- Rust formatting and Clippy static analysis passed.
- TypeScript and Vite production builds passed.
- Tauri production compilation passed on macOS, Windows, and Ubuntu.
- Windows and Linux distribution-package workflows passed.
- English default display, Japanese switching, and language persistence were
  verified in the UI.

### Distribution note

The generated packages are currently unsigned. Operating systems may display a
security warning until platform-specific code signing and, on macOS,
notarization are configured.

---

## 日本語

SpeedReplace v1.0.0は、高速かつ安全なクロスプラットフォーム対応テキスト一括置換アプリの最初の安定版です。

### 主な機能

- 個別ファイルまたはフォルダツリー全体を対象に置換
- 通常文字列、正規表現、複数行の置換ルール
- 有効な複数ルールを指定順に適用
- 統一形式の差分プレビューと変更対象ファイルの選択
- UTF-8、UTF-16、Shift_JIS、BOM、改行コードの保持
- 対象・除外パスのフィルターとバイナリファイルの自動除外
- 書き込み前の検証付きバックアップと安全なUndo
- 古いプレビューや外部変更を検出し、誤った上書きを防止
- シンプルモードと詳細モードの切り替え
- 英語と日本語のUI。デフォルトは英語で、型検査付き言語ファイルによる言語追加に対応

### 配布パッケージ

- Windows x64：MSIおよびNSIS（`.exe`）インストーラー
- Linux x64：Debian（`.deb`）およびAppImageパッケージ
- macOS Apple Silicon：ソースからアプリバンドルとDMGを作成可能

### 検証結果

- Rustエンジンと安全性テスト12件が合格
- Rustフォーマット検査とClippy静的解析が合格
- TypeScriptおよびVite製品ビルドが合格
- macOS、Windows、UbuntuでTauri製品コンパイルが合格
- WindowsおよびLinuxの配布パッケージ生成が合格
- UIの英語初期表示、日本語切り替え、言語設定保持を確認済み

### 配布時の注意

生成されるパッケージは現在未署名です。各プラットフォーム向けのコード署名と、macOSのNotarizationを設定するまで、OSがセキュリティ警告を表示する場合があります。
