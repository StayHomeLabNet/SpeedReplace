# SpeedReplace

[English](README.md) | [日本語](README.ja.md)

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

## v1.0の機能

- ファイルおよびフォルダの再帰的な処理
- 通常文字列、正規表現、複数行の置換
- 複数の置換ルールを指定順に適用
- 対象・除外ファイルのフィルターとバイナリファイル検出
- UTF-8、UTF-16、日本語レガシーエンコーディング対応
- 文字コード、BOM、改行コードの保持
- ファイル単位の差分プレビューとドライラン
- 自動バックアップと競合検出付きUndo
- ドラッグ＆ドロップ
- シンプルモードと詳細モード

実装仕様は[docs/DESIGN_V1.md](docs/DESIGN_V1.md)を参照してください。

## プロジェクトの状態

SpeedReplace v1.0は実装済みです。独立したRustエンジン、Tauriコマンド層、ReactデスクトップUI、検証付きのバックアップ・適用・Undoフロー、クロスプラットフォームCIを含みます。操作方法と安全機能の詳細は[ユーザーガイド](docs/USER_GUIDE.md)を参照してください。

## 開発

必要環境：Node.js 24以上、pnpm 11以上、Rust stableツールチェーン、および対象OSに必要なTauriのネイティブ依存関係。

```bash
pnpm install
cargo test --workspace
pnpm --dir apps/desktop build
pnpm --dir apps/desktop tauri dev
```

ローカルで製品版バンドルを作成する場合：

```bash
pnpm --dir apps/desktop tauri build
```

Windows版とLinux版の配布パッケージは、GitHub Actionsの **Package desktop apps** ワークフローで作成します。手動で実行するか、`v*`タグをプッシュした後、次のArtifactをダウンロードしてください。

- `SpeedReplace-Windows-x64`：MSIおよびNSIS（`.exe`）インストーラー
- `SpeedReplace-Linux-x64`：Debian（`.deb`）およびAppImageパッケージ

ワークフローのArtifactは30日間保持されます。

本ソフトウェアはMIT Licenseの下で公開されています。一般向けにバイナリを配布するには、配布者が保有する各プラットフォームの署名・Notarization用証明書が必要です。
