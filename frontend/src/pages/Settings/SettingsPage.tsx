import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useProjectStore } from "@/stores";
import { LanguageSelector } from "@/components/LanguageSelector/LanguageSelector";
import { EditorSettings } from "@/components/EditorSettings/EditorSettings";
import { GitHubSettings } from "@/components/GitHubSettings/GitHubSettings";
import { BrddSettings } from "@/components/BrddPanel/BrddSettings";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";

const SECTIONS = [
  {
    id: "language",
    icon: "globe",
    labelKey: "settings.language",
    hintKey: "settings.languageHint",
  },
  {
    id: "editors",
    icon: "code",
    labelKey: "settings.editors",
    hintKey: "settings.editorsHint",
  },
  {
    id: "github",
    icon: "github",
    labelKey: "settings.github",
    hintKey: "settings.githubHint",
  },
  {
    id: "brdd",
    icon: "info",
    labelKey: "settings.brdd",
    hintKey: "settings.brddHint",
  },
  {
    id: "roots",
    icon: "folder",
    labelKey: "settings.projectRoots",
    hintKey: "settings.projectRootsHint",
  },
  {
    id: "about",
    icon: "info",
    labelKey: "settings.about",
    hintKey: "settings.aboutText",
  },
] as const;

type SectionId = (typeof SECTIONS)[number]["id"];

export function SettingsPage() {
  const { t } = useTranslation();
  const { roots, fetchRoots } = useProjectStore();
  const [activeSection, setActiveSection] = useState<SectionId>("language");

  useEffect(() => {
    fetchRoots();
  }, [fetchRoots]);

  const active = SECTIONS.find((s) => s.id === activeSection) ?? SECTIONS[0];

  return (
    <div className="page settings">
      <header className="page-header">
        <div>
          <h1>{t("settings.title")}</h1>
          <p className="page-subtitle">{t("settings.subtitle")}</p>
        </div>
      </header>

      <div className="settings-shell">
        <nav className="settings-nav" aria-label={t("nav.settings")}>
          {SECTIONS.map((section) => (
            <button
              key={section.id}
              type="button"
              className={`settings-nav-link${activeSection === section.id ? " active" : ""}`}
              aria-current={activeSection === section.id ? "page" : undefined}
              onClick={() => setActiveSection(section.id)}
            >
              <Icon name={section.icon} size={15} />
              <span>{t(section.labelKey)}</span>
              {section.id === "roots" && roots.length > 0 && (
                <span className="badge">{roots.length}</span>
              )}
            </button>
          ))}
        </nav>

        <section
          className="panel settings-panel"
          aria-labelledby={`settings-panel-title-${active.id}`}
        >
          <header className="panel-header">
            <div>
              <h2 id={`settings-panel-title-${active.id}`}>{t(active.labelKey)}</h2>
              <p>{t(active.hintKey)}</p>
            </div>
          </header>

          <div className="panel-body" key={active.id}>
            {active.id === "language" && <LanguageSelector />}

            {active.id === "editors" && <EditorSettings />}

            {active.id === "github" && <GitHubSettings compact />}

            {active.id === "brdd" && <BrddSettings />}

            {active.id === "roots" &&
              (roots.length === 0 ? (
                <div className="empty-state">
                  <Icon name="folder" size={28} />
                  <p>{t("settings.noRoots")}</p>
                  <Link to="/projects" className="btn primary">
                    {t("settings.goToProjects")}
                  </Link>
                </div>
              ) : (
                <>
                  <p className="settings-section-label">
                    {t("settings.rootsCount", { count: roots.length })}
                  </p>
                  <ul className="path-list">
                    {roots.map((r) => (
                      <li key={r.id}>
                        <Icon name="folder" size={14} />
                        <code title={r.path}>{r.path}</code>
                      </li>
                    ))}
                  </ul>
                  <div className="button-row">
                    <Link to="/projects" className="btn">
                      {t("settings.goToProjects")}
                    </Link>
                  </div>
                </>
              ))}

            {active.id === "about" && (
              <>
                <div className="about">
                  <img src="/logo.png" alt="" width={48} height={48} />
                  <div>
                    <p className="about-name">Project Hub</p>
                    <p className="muted">{t("settings.aboutText")}</p>
                  </div>
                </div>
                <div className="about-tags">
                  <span className="badge accent">v0.1.0</span>
                  <span className="badge">Local-first</span>
                  <span className="badge outline">Tauri + React</span>
                </div>
              </>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
