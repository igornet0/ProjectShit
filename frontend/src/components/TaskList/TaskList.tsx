import { Link } from "react-router-dom";
import type { Recurrence, Task } from "@/types";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";
import { useProjectStore, useUiStore } from "@/stores";
import { formatDate } from "@/utils/format";

interface TaskListProps {
  tasks: Task[];
  onToggle: (task: Task) => void;
  loading?: boolean;
  /** Show which project (or "personal") each task belongs to. */
  showProject?: boolean;
}

const PRIORITY_BADGE: Record<Task["priority"], string> = {
  low: "badge",
  medium: "badge outline",
  high: "badge warning",
  critical: "badge danger",
};

const STATUS_BADGE: Partial<Record<Task["status"], string>> = {
  in_progress: "badge accent",
  cancelled: "badge",
};

export function TaskList({ tasks, onToggle, loading, showProject = false }: TaskListProps) {
  const { t, locale } = useTranslation();
  const projects = useProjectStore((s) => s.projects);
  const openTask = useUiStore((s) => s.openTask);
  const projectName = (id: string | null) =>
    id === null ? null : (projects.find((p) => p.id === id)?.name ?? null);

  if (loading) return <p className="panel-empty">{t("tasks.loading")}</p>;
  if (tasks.length === 0) return <p className="panel-empty">{t("tasks.empty")}</p>;

  const now = Date.now();

  return (
    <ul className="task-list">
      {tasks.map((task) => {
        const closed = task.status === "done" || task.status === "cancelled";
        const overdue = !closed && task.due_at !== null && new Date(task.due_at).getTime() < now;
        const statusBadge = STATUS_BADGE[task.status];
        return (
          <li key={task.id} className={`task-item status-${task.status}`}>
            <input
              type="checkbox"
              checked={task.status === "done"}
              aria-label={task.title}
              onChange={() => onToggle(task)}
            />
            <button
              type="button"
              className={`task-title${closed ? " done" : ""}`}
              onClick={() => openTask(task.id)}
            >
              {task.title}
            </button>
            {showProject &&
              (task.project_id === null ? (
                <span className="badge outline">{t("tasks.personal")}</span>
              ) : (
                <Link to={`/projects/${task.project_id}`} className="task-project">
                  {projectName(task.project_id) ?? "—"}
                </Link>
              ))}
            {task.recurrence && (
              <span
                className="task-repeat"
                title={
                  task.recurrence.until
                    ? `${t("tasks.repeatUntil")} ${formatDate(task.recurrence.until, locale)}`
                    : t("tasks.repeatNoEnd")
                }
              >
                <Icon name="refresh" size={12} />
                {recurrenceLabel(task.recurrence, t)}
              </span>
            )}
            {statusBadge && (
              <span className={statusBadge}>{t(`enums.taskStatus.${task.status}`)}</span>
            )}
            <span
              className={PRIORITY_BADGE[task.priority]}
              title={t("tasks.priority")}
            >
              {t(`enums.taskPriority.${task.priority}`)}
            </span>
            {task.due_at && (
              <span
                className={`task-due${overdue ? " overdue" : ""}`}
                title={t("tasks.dueDate")}
              >
                <Icon name="calendar" size={12} />
                {formatDate(task.due_at, locale, { month: "short", day: "numeric" })}
              </span>
            )}
          </li>
        );
      })}
    </ul>
  );
}

export function recurrenceLabel(
  rule: Recurrence,
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  if (rule.interval <= 1) return t(`enums.recurrence.${rule.frequency}`);
  return t("tasks.repeatEveryN", {
    count: rule.interval,
    unit: t(`enums.recurrenceUnit.${rule.frequency}`),
  });
}
