export const SUPPORTED_LOCALES = [
  "en",
  "ru",
  "zh",
  "ja",
  "es",
  "de",
  "fr",
  "pt",
  "ko",
  "ar",
] as const;

export type Locale = (typeof SUPPORTED_LOCALES)[number];

export interface LanguageOption {
  code: Locale;
  nativeName: string;
  englishName: string;
  flag: string;
  dir: "ltr" | "rtl";
}

export const LANGUAGES: LanguageOption[] = [
  { code: "en", nativeName: "English", englishName: "English", flag: "🇺🇸", dir: "ltr" },
  { code: "ru", nativeName: "Русский", englishName: "Russian", flag: "🇷🇺", dir: "ltr" },
  { code: "zh", nativeName: "简体中文", englishName: "Chinese", flag: "🇨🇳", dir: "ltr" },
  { code: "ja", nativeName: "日本語", englishName: "Japanese", flag: "🇯🇵", dir: "ltr" },
  { code: "es", nativeName: "Español", englishName: "Spanish", flag: "🇪🇸", dir: "ltr" },
  { code: "de", nativeName: "Deutsch", englishName: "German", flag: "🇩🇪", dir: "ltr" },
  { code: "fr", nativeName: "Français", englishName: "French", flag: "🇫🇷", dir: "ltr" },
  { code: "pt", nativeName: "Português", englishName: "Portuguese", flag: "🇧🇷", dir: "ltr" },
  { code: "ko", nativeName: "한국어", englishName: "Korean", flag: "🇰🇷", dir: "ltr" },
  { code: "ar", nativeName: "العربية", englishName: "Arabic", flag: "🇸🇦", dir: "rtl" },
];

export const DEFAULT_LOCALE: Locale = "en";
export const LOCALE_STORAGE_KEY = "project-hub-locale";

export function isLocale(value: string): value is Locale {
  return SUPPORTED_LOCALES.includes(value as Locale);
}

export function detectBrowserLocale(): Locale {
  const langs = navigator.languages?.length
    ? navigator.languages
    : [navigator.language];

  for (const lang of langs) {
    const base = lang.split("-")[0].toLowerCase();
    if (isLocale(base)) return base;
  }
  return DEFAULT_LOCALE;
}

export function getLanguageMeta(code: Locale): LanguageOption {
  return LANGUAGES.find((l) => l.code === code) ?? LANGUAGES[0];
}
