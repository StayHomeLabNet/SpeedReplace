# Localization

SpeedReplace uses a small, type-checked localization layer with English as the
default language. The selected language is stored in local storage under
`speedreplace.language`.

## Adding a language

1. Copy `apps/desktop/src/locales/ja.ts` to a new file in the same directory.
2. Set `code` to a valid BCP 47 language code and `name` to the language's
   native display name.
3. Translate every message while keeping the keys and `{placeholders}` intact.
4. Run `pnpm --dir apps/desktop build`.

Vite automatically discovers every `apps/desktop/src/locales/*.ts` file, so no
central registry needs to be edited. TypeScript reports missing or unknown
translation keys during the build. When a translation is unavailable at
runtime, the English message is used as the fallback.

---

# 多言語化

SpeedReplaceは、英語をデフォルトとする軽量で型検査付きの多言語化機構を使用しています。選択した言語はローカルストレージの`speedreplace.language`に保存されます。

## 言語の追加方法

1. `apps/desktop/src/locales/ja.ts`を同じディレクトリ内に別名でコピーします。
2. `code`に有効なBCP 47言語コード、`name`にその言語の自称を設定します。
3. キーと`{placeholders}`を変更せず、すべてのメッセージを翻訳します。
4. `pnpm --dir apps/desktop build`を実行します。

Viteが`apps/desktop/src/locales/*.ts`を自動検出するため、中央レジストリの変更は不要です。翻訳キーの不足や不明なキーは、ビルド中にTypeScriptが検出します。実行時に翻訳が取得できない場合は英語へフォールバックします。
