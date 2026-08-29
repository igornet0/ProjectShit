import { create } from "zustand";
import en from "./locales/en.json";
import ru from "./locales/ru.json";
import zh from "./locales/zh.json";
import ja from "./locales/ja.json";
import es from "./locales/es.json";
import de from "./locales/de.json";
import fr from "./locales/fr.json";
import pt from "./locales/pt.json";
import ko from "./locales/ko.json";
import ar from "./locales/ar.json";
import {
  DEFAULT_LOCALE,
  LOCALE_STORAGE_KEY,
  detectBrowserLocale,
  getLanguageMeta,
  isLocale,
  type Locale,
} from "./languages";

export type TranslationDict = typeof en;

const catalogs: Record<Locale, TranslationDict> = {
  en,
  ru,
  zh,
  ja,
  es,
  de,
  fr,
  pt,
  ko,
  ar,
};

function getNestedValue(obj: Record<string, unknown>, path: string): string | undefined {
  const parts = path.split(".");
  let current: unknown = obj;
  for (const part of parts) {
    if (current == null || typeof current !== "object") return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return typeof current === "string" ? current : undefined;
}

function interpolate(
  template: string,
  params?: Record<string, string | number>,
): string {
  if (!params) return template;
  return template.replace(/\{\{(\w+)\}\}/g, (_, key: string) =>
    params[key] !== undefined ? String(params[key]) : `{{${key}}}`,
  );
}

interface LocaleState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

function applyDocumentLocale(locale: Locale) {
  const meta = getLanguageMeta(locale);
  document.documentElement.lang = locale;
  document.documentElement.dir = meta.dir;
}

function loadInitialLocale(): Locale {
  const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
  if (stored && isLocale(stored)) return stored;
  return detectBrowserLocale();
}

export const useLocaleStore = create<LocaleState>((set, get) => {
  const initial = loadInitialLocale();
  applyDocumentLocale(initial);

  return {
    locale: initial,
    setLocale: (locale) => {
      localStorage.setItem(LOCALE_STORAGE_KEY, locale);
      applyDocumentLocale(locale);
      set({ locale });
    },
    t: (key, params) => {
      const { locale } = get();
      const dict = catalogs[locale] ?? catalogs[DEFAULT_LOCALE];
      const value =
        getNestedValue(dict as unknown as Record<string, unknown>, key) ??
        getNestedValue(catalogs.en as unknown as Record<string, unknown>, key) ??
        key;
      return interpolate(value, params);
    },
  };
});

export function useTranslation() {
  const locale = useLocaleStore((s) => s.locale);
  const setLocale = useLocaleStore((s) => s.setLocale);
  const t = useLocaleStore((s) => s.t);
  return { locale, setLocale, t };
}

export function useEnumLabel(
  group: "language" | "status" | "projectType",
  value: string,
): string {
  const t = useLocaleStore((s) => s.t);
  const key = `enums.${group}.${value}`;
  const translated = t(key);
  return translated === key ? value : translated;
}

export { LANGUAGES, type Locale } from "./languages";
