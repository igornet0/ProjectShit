import { NavLink, Outlet } from "react-router-dom";
import { CommandPalette } from "@/components/CommandPalette/CommandPalette";
import { TaskDialog } from "@/components/TaskDialog/TaskDialog";
import { Icon, type IconName } from "@/components/Icon/Icon";
import { useProjectStore, useUiStore } from "@/stores";
import { useTranslation } from "@/i18n";

const NAV_ITEMS: Array<{
  to: string;
  end?: boolean;
  icon: IconName;
  key: string;
}> = [
  { to: "/", end: true, icon: "dashboard", key: "nav.dashboard" },
  { to: "/projects", icon: "folder", key: "nav.projects" },
  { to: "/tasks", icon: "tasks", key: "nav.tasks" },
  { to: "/calendar", icon: "calendar", key: "nav.calendar" },
];

export function AppLayout() {
  const { t } = useTranslation();
  const setPaletteOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const projectCount = useProjectStore((s) => s.projects.length);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img src="/logo.png" alt="" className="brand-logo" />
          <span className="brand-name">Project Hub</span>
        </div>

        <button
          type="button"
          className="sidebar-search"
          onClick={() => setPaletteOpen(true)}
          title={t("nav.commandHint", { shortcut: t("common.shortcut") })}
        >
          <Icon name="search" size={14} />
          <span>{t("nav.search")}</span>
          <kbd>{t("common.shortcut")}</kbd>
        </button>

        <nav className="nav">
          {NAV_ITEMS.map(({ to, end, icon, key }) => (
            <NavLink key={to} to={to} end={end} className="nav-link" title={t(key)}>
              <Icon name={icon} />
              <span>{t(key)}</span>
              {to === "/projects" && projectCount > 0 && (
                <span className="nav-link-count">{projectCount}</span>
              )}
            </NavLink>
          ))}
        </nav>

        <nav className="nav" style={{ marginTop: "auto" }}>
          <NavLink to="/settings" className="nav-link" title={t("nav.settings")}>
            <Icon name="settings" />
            <span>{t("nav.settings")}</span>
          </NavLink>
        </nav>

        <div className="sidebar-footer">
          <span>v0.1</span>
          <span>local-first</span>
        </div>
      </aside>

      <main className="content">
        <Outlet />
      </main>
      <CommandPalette />
      <TaskDialog />
    </div>
  );
}
