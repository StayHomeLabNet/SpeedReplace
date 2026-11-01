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
