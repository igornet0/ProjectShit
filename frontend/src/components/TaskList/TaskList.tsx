import type { Task } from "@/types";
import { useTranslation, useLocaleStore } from "@/i18n";

interface TaskListProps {
  tasks: Task[];
  onToggle: (task: Task) => void;
  loading?: boolean;
}

export function TaskList({ tasks, onToggle, loading }: TaskListProps) {
  const { t } = useTranslation();
  const locale = useLocaleStore((s) => s.locale);

  if (loading) return <div className="task-list loading">{t("tasks.loading")}</div>;

  if (tasks.length === 0) {
    return <div className="task-list empty">{t("tasks.empty")}</div>;
  }

  return (
    <ul className="task-list">
      {tasks.map((task) => (
        <li key={task.id} className={`task-item status-${task.status}`}>
          <label>
            <input
              type="checkbox"
              checked={task.status === "done"}
              onChange={() => onToggle(task)}
            />
            <span className={task.status === "done" ? "done" : ""}>
              {task.title}
            </span>
          </label>
          {task.due_at && (
            <span className="task-due">
              {formatDate(task.due_at, locale)}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

function formatDate(iso: string, locale: string): string {
  return new Date(iso).toLocaleDateString(locale, {
    month: "short",
    day: "numeric",
  });
}
