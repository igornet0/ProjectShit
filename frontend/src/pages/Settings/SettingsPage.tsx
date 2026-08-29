import { useEffect } from "react";
import { useProjectStore } from "@/stores";
import { LanguageSelector } from "@/components/LanguageSelector/LanguageSelector";
import { EditorSettings } from "@/components/EditorSettings/EditorSettings";
import { GitHubSettings } from "@/components/GitHubSettings/GitHubSettings";
import { useTranslation } from "@/i18n";

export function SettingsPage() {
  const { t } = useTranslation();
  const { roots, fetchRoots } = useProjectStore();

  useEffect(() => {
    fetchRoots();
  }, [fetchRoots]);

  return (
    <div className="page settings">
      <header className="page-header hero-header">
        <p className="eyebrow">{t("nav.settings")}</p>
        <h1>{t("settings.title")}</h1>
      </header>

      <section className="panel settings-section">
        <h2>{t("settings.language")}</h2>
        <p className="muted">{t("settings.languageHint")}</p>
        <LanguageSelector />
      </section>

      <section className="panel settings-section">
        <h2>{t("settings.editors")}</h2>
        <p className="muted">{t("settings.editorsHint")}</p>
        <EditorSettings />
      </section>

      <section className="panel settings-section">
        <h2>{t("settings.github")}</h2>
        <GitHubSettings />
      </section>

      <section className="panel settings-section">
        <h2>{t("settings.projectRoots")}</h2>
        <p className="muted">{t("settings.projectRootsHint")}</p>
        {roots.length === 0 ? (
          <p>{t("settings.noRoots")}</p>
        ) : (
          <ul className="simple-list">
            {roots.map((r) => (
              <li key={r.id}>
                <code>{r.path}</code>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="panel settings-section">
        <h2>{t("settings.about")}</h2>
        <p>{t("settings.aboutText")}</p>
      </section>
    </div>
  );
}
