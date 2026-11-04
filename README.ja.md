# ⚡ SpeedReplace

## 高速かつ安全なクロスプラットフォーム対応一括テキスト置換ツール

**SpeedReplaceは変更を適用する前に差分をプレビューし、文字コードと改行コードを維持しながら、安全なUndoのための検証済みバックアップを作成します。**

[English](README.md) | [日本語](README.ja.md)

[機能](#機能) ・ [対応ファイル形式](#対応ファイル形式) ・ [リリース](#リリース) ・ [開発](#開発) ・ [多言語化](#多言語化) ・ [コントリビューションとセキュリティ](#コントリビューションとセキュリティ)

[![CI](https://github.com/StayHomeLabNet/SpeedReplace/actions/workflows/ci.yml/badge.svg)](https://github.com/StayHomeLabNet/SpeedReplace/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/StayHomeLabNet/SpeedReplace)](https://github.com/StayHomeLabNet/SpeedReplace/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-orange)
![App](https://img.shields.io/badge/app-Tauri%202-purple)
![Files](https://img.shields.io/badge/files-plain%20text-brightgreen)
![Encoding](https://img.shields.io/badge/encoding-UTF--8%20%7C%20UTF--16%20%7C%20Japanese%20legacy-lightgrey)

[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20StayHomeLabNet-ff5e5b?style=for-the-badge&logo=kofi&logoColor=white)](https://ko-fi.com/stayhomelabnet)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Support%20StayHomeLabNet-ffdd00?style=for-the-badge&logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/stayhomelabnet)

**SpeedReplaceが役に立ったら、ぜひサポートをご検討ください。**

---

SpeedReplaceは、高速かつ安全にテキストを一括置換するためのクロスプラットフォーム対応デスクトップアプリです。

`speeeeed`のシンプルな操作性に、現代的なファイル形式の取り扱いと誤操作によるデータ喪失を防ぐ安全機能を組み合わせています。

## 製品原則

1. 単発の置換でも迷わず使えること。
2. 大規模なディレクトリツリーでも安全に使えること。
3. プレビューを標準的なワークフローにできる速度であること。
4. デフォルトでローカルかつオフラインで動作すること。

## 技術スタック

- Tauri 2
- Rust製置換エンジン
- ReactおよびTypeScript製ユーザーインターフェース
- Windows、macOS、Linux対応

## 機能

- ファイルおよびフォルダの再帰的な処理
- 通常文字列、正規表現、複数行の置換
- 複数の置換ルールを指定順に適用
- 対象・除外ファイルのフィルターとバイナリファイル検出
- UTF-8、UTF-16、日本語レガシーエンコーディング対応
- 文字コード、BOM、改行コードの保持
- ファイル単位の差分プレビューとドライラン
- ファイル内の変更ブロック（ハンク）単位の適用選択
- スキャン・適用の進捗表示とファイル間での安全なキャンセル
- 再利用可能なルールプリセット
- バックアップ保持世代数の自動管理
- 文字コードの自動判定と手動指定
- 自動バックアップと競合検出付きUndo
- デスクトップ版と同じ安全な置換エンジンを使うCLI
- ドラッグ＆ドロップ
- シンプルモードと詳細モード

## 対応ファイル形式

SpeedReplaceはプレーンテキストファイルを処理します。対応可否は拡張子だけではなく、ファイルの内容と文字コードで判定されます。主な例は次の通りです。

- テキスト・文書：`.txt`、`.csv`、`.tsv`、`.log`、`.md`、`.rst`、`.adoc`、`.tex`
- 構造化データ・マークアップ：`.json`、`.jsonl`、`.xml`、`.html`、`.htm`、`.yaml`、`.yml`、`.toml`、`.sql`、`.graphql`、`.gql`
- Web・テンプレート：`.php`、`.phtml`、`.php5`、`.css`、`.scss`、`.sass`、`.less`、`.vue`、`.svelte`、`.astro`、`.twig`、`.blade.php`、`.erb`
- 設定・ビルドファイル：`.ini`、`.cfg`、`.conf`、`.properties`、`.env`、`.editorconfig`、`.gitignore`、`.gradle`、`.cmake`、`.mk`、拡張子のない`Dockerfile`や`Makefile`
- ソースコード・スクリプト：`.rs`、`.js`、`.ts`、`.tsx`、`.jsx`、`.py`、`.rb`、`.java`、`.c`、`.cpp`、`.h`、`.cs`、`.go`、`.swift`、`.kt`、`.kts`、`.scala`、`.lua`、`.pl`、`.pm`、`.r`、`.dart`、`.ex`、`.exs`、`.erl`、`.hrl`、`.fs`、`.fsx`、`.vb`、`.sh`、`.ps1`、`.bat`、`.cmd`

これらは例示であり、拡張子の許可リストではありません。その他の拡張子のないファイルや独自拡張子でも、対応文字コードのテキストで検証に合格すれば処理できます。

対応文字コードはUTF-8、UTF-16 LE/BE、Shift_JIS、EUC-JP、ISO-2022-JP、Windows-1252です。UTF-8とUTF-16のBOMは維持されます。置換を行う前に、厳密なデコードと往復変換の検証に合格する必要があります。

`.docx`、`.xlsx`、`.pdf`、圧縮ファイル、データベース、画像、音声、動画などのバイナリ・コンテナ形式には対応していません。非対応ファイルはデフォルトでプレビューから非表示となり、**非対応ファイルを表示**で確認できます。構造化形式もテキストとして処理され、CSV、JSON、XML、ソースコードの構文解析は行いません。

当初の設計ベースラインとロードマップは[docs/DESIGN_V1.md](docs/DESIGN_V1.md)を参照してください。

## プロジェクトの状態

SpeedReplace v1.1.0は現在の安定版です。独立したRustエンジン、Tauriコマンド層、ReactデスクトップUI、検証付きのバックアップ・適用・Undoフロー、クロスプラットフォームCIを含みます。操作方法と安全機能の詳細は[ユーザーガイド](docs/USER_GUIDE.md)を参照してください。

## リリース

現在の安定版は **v1.1.0** です。機能、対応パッケージ、検証結果、配布時の注意事項は、英語・日本語併記の[リリースノート](RELEASE_NOTES.md)を参照してください。インストーラーは[v1.1.0 Release](https://github.com/StayHomeLabNet/SpeedReplace/releases/tag/v1.1.0)からダウンロードし、添付の`SHA256SUMS.txt`で検証できます。

## 開発

必要環境：Node.js 24以上、pnpm 11以上、Rust stableツールチェーン、および対象OSに必要なTauriのネイティブ依存関係。

```bash
pnpm install
cargo test --workspace
pnpm --dir apps/desktop build
pnpm --dir apps/desktop tauri dev
```

CLIのビルドと基本的な使い方：

```bash
cargo build --release -p speedreplace-cli
speedreplace scan ./src --search old --replacement new
speedreplace replace ./src --search old --replacement new --retain 20
speedreplace history --backup-dir .speedreplace
speedreplace undo <operation-id> --backup-dir .speedreplace
```

`scan`は読み取り専用です。`replace`は検証済みバックアップを作成してから変更します。必要に応じて`--regex`、`--include`、`--exclude`、`--encoding`、`--rules rules.json`を指定できます。全オプションは`speedreplace <command> --help`で確認してください。

ローカルで製品版バンドルを作成する場合：

```bash
pnpm --dir apps/desktop tauri build
```

Windows版、Linux版、macOS版の配布パッケージは、GitHub Actionsの **Package desktop apps** ワークフローで作成します。手動で実行するか、`v*`タグをプッシュした後、次のArtifactをダウンロードしてください。

- `SpeedReplace-Windows-x64`：MSIおよびNSIS（`.exe`）インストーラー
- `SpeedReplace-Linux-x64`：Debian（`.deb`）およびAppImageパッケージ
- `SpeedReplace-macOS-Universal`：Apple SiliconとIntel Macの両方に対応するUniversal DMG

各Artifactには対象OS用の`speedreplace` CLI（Windowsでは`speedreplace.exe`）も含まれます。macOS CLIはUniversalバイナリです。

ワークフローのArtifactは30日間保持されます。

## 多言語化

UIのデフォルト言語は英語で、アプリ上部から言語を切り替えられます。新しい言語は、自動検出される型検査付き言語ファイルとして追加できます。詳細は[docs/LOCALIZATION.md](docs/LOCALIZATION.md)を参照してください。

## コントリビューションとセキュリティ

プルリクエストを開く前に[CONTRIBUTING.md](CONTRIBUTING.md)を確認してください。脆弱性の疑いは、[SECURITY.md](SECURITY.md)に従って非公開で報告してください。

本ソフトウェアはMIT Licenseの下で公開されています。一般向けにバイナリを配布するには、配布者が保有する各プラットフォームの署名・Notarization用証明書が必要です。
