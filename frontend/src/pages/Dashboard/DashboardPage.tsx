import { useEffect } from "react";
import { Link } from "react-router-dom";
import { Icon, type IconName } from "@/components/Icon/Icon";
import { TaskList } from "@/components/TaskList/TaskList";
import { useProjectStore, useTaskStore, useCalendarStore } from "@/stores";
import { useTranslation } from "@/i18n";
import { formatRelative, languageColor, shortPath } from "@/utils/format";

export function DashboardPage() {
  const { t, locale } = useTranslation();
  const { projects, fetchProjects, loading, markOpened } = useProjectStore();
  const { tasks, fetchTasks, toggleTask } = useTaskStore();
  const { events, fetchEvents } = useCalendarStore();

  useEffect(() => {
    fetchProjects();
    fetchTasks();
    fetchEvents();
  }, [fetchProjects, fetchTasks, fetchEvents]);

  const activeProjects = projects.filter(
    (p) => p.status === "active" && p.depth === 0,
  );
  const dueTasks = tasks
    .filter((task) => task.status !== "done" && task.due_at)
    .sort((a, b) => (a.due_at ?? "").localeCompare(b.due_at ?? ""));
  const todayEvents = events.filter((e) => isToday(e.start_at));
  const recent = projects.filter((p) => p.depth === 0).slice(0, 8);

  const stats: Array<{ icon: IconName; label: string; value: number }> = [
    { icon: "folder", label: t("dashboard.activeProjects"), value: activeProjects.length },
    { icon: "tasks", label: t("dashboard.tasksDue"), value: dueTasks.length },
    { icon: "calendar", label: t("dashboard.eventsToday"), value: todayEvents.length },
  ];

  return (
    <div className="page dashboard">
      <header className="page-header">
        <div>
          <h1>{getGreeting(t)}</h1>
          <p className="page-subtitle">{t("dashboard.subtitle")}</p>
        </div>
      </header>

      <div className="stats-row">
        {stats.map((s) => (
          <div key={s.label} className="panel stat">
            <span className="stat-label">
              <Icon name={s.icon} size={14} />
              {s.label}
            </span>
            <span className="stat-value">{s.value}</span>
          </div>
        ))}
      </div>

      <div className="dashboard-grid">
        <section className="panel">
          <div className="panel-header">
            <h2>
              <Icon name="clock" size={14} />
              {t("dashboard.recentProjects")}
            </h2>
            <Link to="/projects">{t("common.viewAll")}</Link>
          </div>
          {loading && recent.length === 0 ? (
            <p className="panel-empty">{t("common.loading")}</p>
          ) : recent.length === 0 ? (
            <p className="panel-empty">{t("projects.emptyTitle")}</p>
          ) : (
            <div className="list">
              {recent.map((p) => (
                <Link
                  key={p.id}
                  to={`/projects/${p.id}`}
                  className="list-row"
                  onClick={() => markOpened(p.id)}
                >
                  <span
                    className="dot"
                    style={{ "--dot-color": languageColor(p.language) } as React.CSSProperties}
                  />
                  <span className="list-row-main">
                    <span className="list-row-title truncate">{p.name}</span>
                    <span className="list-row-sub mono truncate">{shortPath(p.root_path)}</span>
                  </span>
                  {p.git_dirty && (
                    <span className="dirty-dot" title={t("projects.gitDirty")} />
                  )}
                  <span className="list-row-meta">
                    {formatRelative(p.last_opened_at ?? p.last_modified_at ?? p.updated_at, locale)}
                  </span>
                </Link>
              ))}
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-header">
            <h2>
              <Icon name="tasks" size={14} />
              {t("dashboard.tasksDue")}
            </h2>
            <Link to="/tasks">{t("common.viewAll")}</Link>
          </div>
          <TaskList tasks={dueTasks.slice(0, 8)} onToggle={toggleTask} showProject />
        </section>
      </div>
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
