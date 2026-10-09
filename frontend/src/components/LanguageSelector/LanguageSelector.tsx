import { Icon } from "@/components/Icon/Icon";
import { useTranslation, LANGUAGES, type Locale } from "@/i18n";

export function LanguageSelector() {
  const { locale, setLocale } = useTranslation();

  return (
    <ul className="choice-list grid">
      {LANGUAGES.map((lang) => {
        const active = locale === lang.code;
        return (
          <li key={lang.code}>
            <button
              type="button"
              className={`choice${active ? " active" : ""}`}
              onClick={() => setLocale(lang.code as Locale)}
              aria-pressed={active}
            >
              <span className="choice-flag" aria-hidden="true">
                {lang.flag}
              </span>
              <span className="choice-main">
                <span className="choice-title">{lang.nativeName}</span>
                <span className="choice-sub">{lang.englishName}</span>
              </span>
              {active && <Icon name="check" size={15} className="choice-check" />}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
