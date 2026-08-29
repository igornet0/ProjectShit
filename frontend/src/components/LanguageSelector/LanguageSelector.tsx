import { useTranslation, LANGUAGES, type Locale } from "@/i18n";

export function LanguageSelector() {
  const { locale, setLocale } = useTranslation();

  return (
    <div className="language-grid">
      {LANGUAGES.map((lang) => (
        <button
          key={lang.code}
          type="button"
          className={`language-card ${locale === lang.code ? "active" : ""}`}
          onClick={() => setLocale(lang.code as Locale)}
          aria-pressed={locale === lang.code}
        >
          <span className="language-flag">{lang.flag}</span>
          <span className="language-native">{lang.nativeName}</span>
          <span className="language-english">{lang.englishName}</span>
        </button>
      ))}
    </div>
  );
}
