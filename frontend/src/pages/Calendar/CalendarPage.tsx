import { useEffect, useMemo, useState } from "react";
import { CalendarView } from "@/components/Calendar/CalendarView";
import { Icon } from "@/components/Icon/Icon";
import { useCalendarStore, useProjectStore, useTaskStore } from "@/stores";
import { useTranslation } from "@/i18n";
import { TASK_PRIORITIES, TASK_STATUSES, type Task } from "@/types";
import { occurrencesBetween } from "@/utils/recurrence";

/** "all", "personal", or a project id. */
type ProjectFilter = string;
/** "all", "open" (not done/cancelled), or a concrete status. */
type StatusFilter = "all" | "open" | Task["status"];
type PriorityFilter = "all" | Task["priority"];

export function CalendarPage() {
  const { t } = useTranslation();
  const { events, fetchEvents, loading } = useCalendarStore();
  const { tasks, fetchTasks } = useTaskStore();
  const { projects, fetchProjects } = useProjectStore();

  const [month, setMonth] = useState(() => startOfMonth(new Date()));
  const [projectFilter, setProjectFilter] = useState<ProjectFilter>("all");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("open");
  const [priorityFilter, setPriorityFilter] = useState<PriorityFilter>("all");
  const [showEvents, setShowEvents] = useState(true);

  useEffect(() => {
    fetchEvents();
    fetchTasks();
    fetchProjects();
  }, [fetchEvents, fetchTasks, fetchProjects]);

  const visibleTasks = useMemo(
    () =>
      tasks.filter((task) => {
        if (!task.due_at) return false;
        if (projectFilter === "personal" && task.project_id !== null) return false;
        if (projectFilter !== "all" && projectFilter !== "personal" && task.project_id !== projectFilter)
          return false;
        if (statusFilter === "open" && (task.status === "done" || task.status === "cancelled"))
          return false;
        if (statusFilter !== "all" && statusFilter !== "open" && task.status !== statusFilter)
          return false;
        if (priorityFilter !== "all" && task.priority !== priorityFilter) return false;
        return true;
      }),
    [tasks, projectFilter, statusFilter, priorityFilter],
  );

  // Future occurrences of open recurring tasks, projected across the visible grid.
  const projected = useMemo(() => {
    const gridStart = new Date(month.getFullYear(), month.getMonth(), 1 - ((month.getDay() + 6) % 7));
    const gridEnd = new Date(gridStart.getFullYear(), gridStart.getMonth(), gridStart.getDate() + 42);
    return visibleTasks
      .filter((task) => task.recurrence && task.due_at && task.status !== "done" && task.status !== "cancelled")
      .flatMap((task) =>
        occurrencesBetween(task.recurrence!, new Date(task.due_at!), gridEnd).map((at) => ({
          task,
          at,
        })),
      );
  }, [visibleTasks, month]);

  const visibleEvents = useMemo(() => {
    if (!showEvents) return [];
    if (projectFilter === "all") return events;
    if (projectFilter === "personal") return events.filter((e) => e.project_id === null);
    return events.filter((e) => e.project_id === projectFilter);
  }, [events, showEvents, projectFilter]);

  const shiftMonth = (delta: number) =>
    setMonth((m) => new Date(m.getFullYear(), m.getMonth() + delta, 1));

  return (
    <div className="page calendar-page">
      <header className="page-header">
        <div>
          <h1>{t("calendar.title")}</h1>
          <p className="page-subtitle">{t("calendar.subtitle")}</p>
        </div>
      </header>

      <div className="toolbar calendar-toolbar">
        <div className="button-group">
          <button
            type="button"
            className="icon-btn"
            onClick={() => shiftMonth(-1)}
            title={t("calendar.prevMonth")}
            aria-label={t("calendar.prevMonth")}
          >
            <Icon name="chevronRight" size={15} className="flip-x" />
          </button>
          <button type="button" className="btn small" onClick={() => setMonth(startOfMonth(new Date()))}>
            {t("calendar.today")}
          </button>
          <button
            type="button"
            className="icon-btn"
            onClick={() => shiftMonth(1)}
            title={t("calendar.nextMonth")}
            aria-label={t("calendar.nextMonth")}
          >
            <Icon name="chevronRight" size={15} />
          </button>
        </div>

        <span className="segmented-sep" />

        <select
          value={projectFilter}
          onChange={(e) => setProjectFilter(e.target.value)}
          aria-label={t("nav.projects")}
          title={t("nav.projects")}
        >
          <option value="all">{t("folders.all")}</option>
          <option value="personal">{t("tasks.personal")}</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>

        <select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
          aria-label={t("tasks.status")}
          title={t("tasks.status")}
        >
          <option value="all">{t("tasks.allStatuses")}</option>
          <option value="open">{t("tasks.openOnly")}</option>
          {TASK_STATUSES.map((s) => (
            <option key={s} value={s}>
              {t(`enums.taskStatus.${s}`)}
            </option>
          ))}
        </select>

        <select
          value={priorityFilter}
          onChange={(e) => setPriorityFilter(e.target.value as PriorityFilter)}
          aria-label={t("tasks.priority")}
          title={t("tasks.priority")}
        >
          <option value="all">{t("tasks.allPriorities")}</option>
          {TASK_PRIORITIES.map((p) => (
            <option key={p} value={p}>
              {t(`enums.taskPriority.${p}`)}
            </option>
          ))}
        </select>

        <span className="toolbar-spacer" />

        <label className="toggle">
          <input
            type="checkbox"
            checked={showEvents}
            onChange={(e) => setShowEvents(e.target.checked)}
          />
          <span>{t("calendar.showEvents")}</span>
        </label>
      </div>

      {loading && events.length === 0 && tasks.length === 0 ? (
        <p className="page-state">{t("common.loading")}</p>
      ) : (
        <CalendarView
          month={month}
          events={visibleEvents}
          tasks={visibleTasks}
          projected={projected}
        />
      )}
    </div>
  );
}

function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}
