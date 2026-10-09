import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { ask } from "@tauri-apps/plugin-dialog";
import { Icon } from "@/components/Icon/Icon";
import { recurrenceLabel } from "@/components/TaskList/TaskList";
import { useProjectStore, useTaskStore, useUiStore } from "@/stores";
import { useTranslation } from "@/i18n";
import { formatDate } from "@/utils/format";
import { TASK_PRIORITIES, TASK_STATUSES, type Task } from "@/types";

/** Global view/edit dialog for one task; opened via `useUiStore().openTask(id)`. */
export function TaskDialog() {
  const openTaskId = useUiStore((s) => s.openTaskId);
  const task = useTaskStore((s) => s.tasks.find((t) => t.id === openTaskId) ?? null);

  if (!openTaskId || !task) return null;
  // Remount per task so the draft resets when another task is opened.
  return <TaskDialogContent key={task.id} task={task} />;
}

function TaskDialogContent({ task }: { task: Task }) {
  const { t, locale } = useTranslation();
  const close = useUiStore((s) => () => s.openTask(null));
  const { updateTask, deleteTask } = useTaskStore();
  const projects = useProjectStore((s) => s.projects);

  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description ?? "");
  const [projectId, setProjectId] = useState(task.project_id ?? "");
  const [status, setStatus] = useState(task.status);
  const [priority, setPriority] = useState(task.priority);
  const [dueDate, setDueDate] = useState(task.due_at ? toDateInput(task.due_at) : "");
  const [stopRepeat, setStopRepeat] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  const project = task.project_id ? projects.find((p) => p.id === task.project_id) : null;

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await updateTask({
        id: task.id,
        title: title.trim(),
        description,
        project_id: projectId,
        status,
        priority,
        ...(dueDate ? { due_at: endOfLocalDay(dueDate) } : { clear_due_at: true }),
        clear_recurrence: stopRepeat,
      });
      close();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    const confirmed = await ask(t("tasks.deleteConfirm", { title: task.title }), {
      title: t("tasks.delete"),
      kind: "warning",
      okLabel: t("common.remove"),
      cancelLabel: t("common.cancel"),
    });
    if (!confirmed) return;
    try {
      await deleteTask(task.id);
      close();
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="dialog-overlay" onMouseDown={close}>
      <form
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-label={task.title}
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={handleSave}
      >
        <div className="dialog-head">
          <input
            className="dialog-title-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            aria-label={t("tasks.newPlaceholder")}
            autoFocus
          />
          <button
            type="button"
            className="icon-btn"
            onClick={close}
            aria-label={t("commandPalette.close")}
          >
            <Icon name="x" size={15} />
          </button>
        </div>

        <div className="dialog-body">
          <div className="form-grid">
            <label className="field">
              <span>{t("tasks.project")}</span>
              <select value={projectId} onChange={(e) => setProjectId(e.target.value)}>
                <option value="">{t("tasks.personal")}</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>

            <label className="field">
              <span>{t("tasks.dueDate")}</span>
              <input type="date" value={dueDate} onChange={(e) => setDueDate(e.target.value)} />
            </label>

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
          </div>

          <label className="field">
            <span>{t("tasks.description")}</span>
            <textarea
              rows={4}
              value={description}
              placeholder={t("tasks.descriptionPlaceholder")}
              onChange={(e) => setDescription(e.target.value)}
            />
          </label>

          {task.recurrence && (
            <div className={`dialog-repeat${stopRepeat ? " stopped" : ""}`}>
              <Icon name="refresh" size={14} />
              <span className="dialog-repeat-text">
                {recurrenceLabel(task.recurrence, t)}
                {" · "}
                {task.recurrence.until
                  ? `${t("tasks.repeatUntil")} ${formatDate(task.recurrence.until, locale)}`
                  : t("tasks.repeatNoEnd")}
              </span>
              <button
                type="button"
                className="btn small ghost"
                aria-pressed={stopRepeat}
                onClick={() => setStopRepeat((v) => !v)}
              >
                {stopRepeat ? t("common.cancel") : t("tasks.stopRepeat")}
              </button>
            </div>
          )}

          {error && <div className="inline-alert">{error}</div>}
        </div>

        <div className="dialog-foot">
          <button type="button" className="btn danger" onClick={handleDelete}>
            <Icon name="trash" size={14} />
            {t("tasks.delete")}
          </button>
          {project && (
            <Link to={`/projects/${project.id}`} className="btn ghost" onClick={close}>
              <Icon name="folder" size={14} />
              {t("tasks.openProject")}
            </Link>
          )}
          <span className="toolbar-spacer" />
          <button type="button" className="btn" onClick={close}>
            {t("common.cancel")}
          </button>
          <button type="submit" className="btn primary" disabled={saving || !title.trim()}>
            {t("common.save")}
          </button>
        </div>
      </form>
    </div>
  );
}

function toDateInput(iso: string): string {
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function endOfLocalDay(date: string): string {
  const [y, m, d] = date.split("-").map(Number);
  return new Date(y, m - 1, d, 23, 59, 59).toISOString();
}
