import { useEffect, useState } from "react";
import { TaskList } from "@/components/TaskList/TaskList";
import { useProjectStore, useTaskStore } from "@/stores";
import { useTranslation } from "@/i18n";

export function TasksPage() {
  const { t } = useTranslation();
  const { tasks, loading, fetchTasks, toggleTask, createTask } = useTaskStore();
  const { projects, fetchProjects } = useProjectStore();
  const [title, setTitle] = useState("");
  const [projectId, setProjectId] = useState("");

  useEffect(() => {
    fetchTasks();
    fetchProjects();
  }, [fetchTasks, fetchProjects]);

  useEffect(() => {
    if (projects.length > 0 && !projectId) {
      setProjectId(projects[0].id);
    }
  }, [projects, projectId]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !projectId) return;
    await createTask({ project_id: projectId, title: title.trim() });
    setTitle("");
  };

  return (
    <div className="page tasks">
      <header className="page-header hero-header">
        <p className="eyebrow">{t("nav.tasks")}</p>
        <h1>{t("tasks.title")}</h1>
      </header>

      <form className="task-form panel" onSubmit={handleCreate}>
        <select
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
        >
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        <input
          placeholder={t("tasks.newPlaceholder")}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <button type="submit" className="btn primary">
          {t("common.add")}
        </button>
      </form>

      <div className="panel">
        <TaskList tasks={tasks} onToggle={toggleTask} loading={loading} />
      </div>
    </div>
  );
}
