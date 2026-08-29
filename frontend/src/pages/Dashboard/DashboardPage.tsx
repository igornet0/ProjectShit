import { useEffect } from "react";
import { Link } from "react-router-dom";
import { useProjectStore, useTaskStore, useCalendarStore } from "@/stores";
import { useTranslation } from "@/i18n";

export function DashboardPage() {
  const { t } = useTranslation();
  const { projects, fetchProjects, loading, markOpened } = useProjectStore();
  const { tasks, fetchTasks } = useTaskStore();
  const { events, fetchEvents } = useCalendarStore();

  useEffect(() => {
    fetchProjects();
    fetchTasks();
    fetchEvents();
  }, [fetchProjects, fetchTasks, fetchEvents]);

  const activeProjects = projects.filter(
    (p) => p.status === "active" && p.depth === 0,
  );
  const dueTasks = tasks.filter((t) => t.status !== "done" && t.due_at);
  const todayEvents = events.filter((e) => isToday(e.start_at));

  return (
    <div className="page dashboard">
      <header className="page-header hero-header">
        <div>
          <p className="eyebrow">{t("nav.dashboard")}</p>
          <h1>{getGreeting(t)}</h1>
          <p className="muted">{t("dashboard.subtitle")}</p>
        </div>
      </header>

      <div className="stats-row">
        <div className="stat-card stat-card--violet">
          <span className="stat-value">{activeProjects.length}</span>
          <span className="stat-label">{t("dashboard.activeProjects")}</span>
        </div>
        <div className="stat-card stat-card--amber">
          <span className="stat-value">{dueTasks.length}</span>
          <span className="stat-label">{t("dashboard.tasksDue")}</span>
        </div>
        <div className="stat-card stat-card--cyan">
          <span className="stat-value">{todayEvents.length}</span>
          <span className="stat-label">{t("dashboard.eventsToday")}</span>
        </div>
      </div>

      <section className="panel">
        <div className="section-header">
          <h2>{t("dashboard.recentProjects")}</h2>
          <Link to="/projects">{t("common.viewAll")}</Link>
        </div>
        {loading ? (
          <p className="muted">{t("common.loading")}</p>
        ) : (
          <ul className="simple-list">
            {projects.filter((p) => p.depth === 0).slice(0, 5).map((p) => (
              <li key={p.id}>
                <Link
                  to={`/projects/${p.id}`}
                  onClick={() => markOpened(p.id)}
                >
                  {p.name}
                </Link>
                <span className="tag">{t(`enums.language.${p.language}`)}</span>
                {p.git_dirty && (
                  <span className="git-warning-inline" title={t("projects.gitDirty")}>
                    ⚠️
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function getGreeting(t: (key: string) => string): string {
  const h = new Date().getHours();
  if (h < 12) return t("greeting.morning");
  if (h < 18) return t("greeting.afternoon");
  return t("greeting.evening");
}

function isToday(iso: string): boolean {
  const d = new Date(iso);
  const now = new Date();
  return (
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate()
  );
}
