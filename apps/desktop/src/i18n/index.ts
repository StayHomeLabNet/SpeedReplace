import { useEffect, useMemo, useState } from "react";
import english, { type TranslationKey } from "../locales/en";
import type { LocaleDefinition, TranslationParams } from "./types";

const STORAGE_KEY = "speedreplace.language";
const localeModules = import.meta.glob<{ default: LocaleDefinition<TranslationKey> }>(
  "../locales/*.ts",
  { eager: true },
);

export const locales = Object.values(localeModules)
  .map((module) => module.default)
  .sort((a, b) => (a.code === "en" ? -1 : b.code === "en" ? 1 : a.name.localeCompare(b.name)));

const localeByCode = new Map(locales.map((locale) => [locale.code, locale]));

function interpolate(message: string, params: TranslationParams = {}) {
  return message.replace(/\{(\w+)\}/g, (match, key: string) =>
    Object.prototype.hasOwnProperty.call(params, key) ? String(params[key]) : match,
  );
}

export function useI18n() {
  const [language, setLanguage] = useState(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored && localeByCode.has(stored) ? stored : "en";
  });

  const locale = localeByCode.get(language) ?? english;

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, locale.code);
    document.documentElement.lang = locale.code;
  }, [locale.code]);

  return useMemo(() => {
    const t = (key: TranslationKey, params?: TranslationParams) =>
      interpolate(locale.messages[key] ?? english.messages[key], params);
    const count = (key: "target" | "file" | "match" | "conflict", value: number) => {
      const form = new Intl.PluralRules(locale.code).select(value) === "one" ? "one" : "other";
      return t(`count.${key}.${form}` as TranslationKey, { count: value });
    };
    return { language: locale.code, setLanguage, locale, locales, t, count };
  }, [locale]);
}

export type { TranslationKey } from "../locales/en";
export type { TranslationParams } from "./types";
