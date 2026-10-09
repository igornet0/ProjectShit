import { useEffect, useState } from "react";
import { TaskList } from "@/components/TaskList/TaskList";
import { Icon } from "@/components/Icon/Icon";
import { useProjectStore, useTaskStore } from "@/stores";
import { useTranslation } from "@/i18n";
import {
  RECURRENCE_FREQUENCIES,
  TASK_PRIORITIES,
  TASK_STATUSES,
  type RecurrenceFrequency,
  type Task,
} from "@/types";

export function TasksPage() {
  const { t } = useTranslation();
  const { tasks, loading, fetchTasks, toggleTask, createTask } = useTaskStore();
  const { projects, fetchProjects } = useProjectStore();
  const [title, setTitle] = useState("");
  const [projectId, setProjectId] = useState("");
  const [status, setStatus] = useState<Task["status"]>("todo");
  const [priority, setPriority] = useState<Task["priority"]>("medium");
  const [dueDate, setDueDate] = useState("");
  const [repeat, setRepeat] = useState<RecurrenceFrequency | "">("");
  const [repeatInterval, setRepeatInterval] = useState(1);
  const [repeatUntil, setRepeatUntil] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    fetchTasks();
    fetchProjects();
  }, [fetchTasks, fetchProjects]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setSubmitting(true);
    try {
      await createTask({
        project_id: projectId || null,
        title: title.trim(),
        status,
        priority,
        due_at: dueDate ? endOfLocalDay(dueDate) : undefined,
        recurrence: repeat
          ? {
              frequency: repeat,
              interval: repeatInterval,
              until: repeatUntil ? endOfLocalDay(repeatUntil) : undefined,
            }
          : undefined,
      });
      setTitle("");
      setDueDate("");
      setRepeat("");
      setRepeatInterval(1);
      setRepeatUntil("");
    } finally {
      setSubmitting(false);
    }
  };

  const open = tasks.filter((task) => task.status !== "done" && task.status !== "cancelled").length;

  return (
    <div className="page tasks">
      <header className="page-header">
        <div>
          <h1>{t("tasks.title")}</h1>
          <p className="page-subtitle">
            {open} / {tasks.length}
          </p>
        </div>
      </header>

      <form className="panel task-form" onSubmit={handleCreate}>
        <div className="task-form-main">
          <select
            value={projectId}
            aria-label={t("nav.projects")}
            title={t("nav.projects")}
            onChange={(e) => setProjectId(e.target.value)}
          >
            <option value="">{t("tasks.personal")}</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <input
            placeholder={t("tasks.newPlaceholder")}
            aria-label={t("tasks.newPlaceholder")}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
        </div>

        <div className="task-form-meta">
          <label className="field">
            <span>{t("tasks.status")}</span>
            <select value={status} onChange={(e) => setStatus(e.target.value as Task["status"])}>
              {TASK_STATUSES.map((s) => (
                <option key={s} value={s}>
                  {t(`enums.taskStatus.${s}`)}
                </option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>{t("tasks.priority")}</span>
            <select
              value={priority}
              onChange={(e) => setPriority(e.target.value as Task["priority"])}
            >
              {TASK_PRIORITIES.map((p) => (
                <option key={p} value={p}>
                  {t(`enums.taskPriority.${p}`)}
                </option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>{t("tasks.dueDate")}</span>
            <input
              type="date"
              value={dueDate}
              required={repeat !== ""}
              onChange={(e) => setDueDate(e.target.value)}
            />
          </label>

          <label className="field">
            <span>{t("tasks.repeat")}</span>
            <select
              value={repeat}
              onChange={(e) => {
                const value = e.target.value as RecurrenceFrequency | "";
                setRepeat(value);
                // A series is anchored at its first due date.
                if (value && !dueDate) setDueDate(todayLocal());
              }}
            >
              <option value="">{t("tasks.repeatNone")}</option>
              {RECURRENCE_FREQUENCIES.map((f) => (
                <option key={f} value={f}>
                  {t(`enums.recurrence.${f}`)}
                </option>
              ))}
            </select>
          </label>

          {repeat && (
            <>
              <label className="field field-narrow">
                <span>{t("tasks.repeatEvery")}</span>
                <span className="input-suffix">
                  <input
                    type="number"
                    min={1}
                    max={365}
                    value={repeatInterval}
                    onChange={(e) =>
                      setRepeatInterval(Math.min(365, Math.max(1, Number(e.target.value) || 1)))
                    }
                  />
                  <span>{t(`enums.recurrenceUnit.${repeat}`)}</span>
                </span>
              </label>

              <label className="field">
                <span>{t("tasks.repeatUntil")}</span>
                <input
                  type="date"
                  value={repeatUntil}
                  min={dueDate || undefined}
                  title={repeatUntil ? undefined : t("tasks.repeatNoEnd")}
                  onChange={(e) => setRepeatUntil(e.target.value)}
                />
              </label>
            </>
          )}

          <button
            type="submit"
            className="btn primary"
            disabled={submitting || !title.trim()}
          >
            <Icon name="plus" size={14} />
            {t("common.add")}
          </button>
        </div>
      </form>

      <div className="panel">
        <TaskList
          tasks={tasks}
          onToggle={toggleTask}
          loading={loading && tasks.length === 0}
          showProject
        />
      </div>
    </div>
  );
}

function todayLocal(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** `YYYY-MM-DD` from a date input → ISO timestamp at the end of that local day. */
function endOfLocalDay(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  return new Date(y, m - 1, d, 23, 59, 59).toISOString();
}
