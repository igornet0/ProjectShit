import { NavLink, Outlet, useLocation } from "react-router-dom";
import { CommandPalette } from "@/components/CommandPalette/CommandPalette";
import { useTranslation } from "@/i18n";

const NAV_ITEMS: Array<{
  to: string;
  end?: boolean;
  icon: string;
  key: string;
}> = [
  { to: "/", end: true, icon: "◈", key: "nav.dashboard" },
  { to: "/projects", icon: "⬡", key: "nav.projects" },
  { to: "/tasks", icon: "☑", key: "nav.tasks" },
  { to: "/calendar", icon: "▦", key: "nav.calendar" },
  { to: "/settings", icon: "⚙", key: "nav.settings" },
];

const PAGE_CLASS: Record<string, string> = {
  "/": "page-theme-dashboard",
  "/projects": "page-theme-projects",
  "/tasks": "page-theme-tasks",
  "/calendar": "page-theme-calendar",
  "/settings": "page-theme-settings",
};

export function AppLayout() {
  const { t } = useTranslation();
  const location = useLocation();

  const basePath =
    location.pathname.startsWith("/projects/") && location.pathname !== "/projects"
      ? "/projects"
      : location.pathname;

  const pageClass = PAGE_CLASS[basePath] ?? "page-theme-default";

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="logo">
          <img src="/logo.png" alt="Project Hub" className="logo-img" />
          <span className="logo-text">Project Hub</span>
        </div>
        <nav>
          {NAV_ITEMS.map(({ to, end, icon, key }) => (
            <NavLink key={to} to={to} end={end} className="nav-link">
              <span className="nav-icon">{icon}</span>
              <span>{t(key)}</span>
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-hint">
          {t("nav.commandHint", { shortcut: t("common.shortcut") })}
        </div>
      </aside>
      <main className={`content ${pageClass}`}>
        <Outlet />
      </main>
      <CommandPalette />
    </div>
  );
}
