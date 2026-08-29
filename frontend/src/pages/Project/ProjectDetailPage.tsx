import { useEffect, useState, useCallback } from "react";
import { useParams, Link, useNavigate } from "react-router-dom";
import { ask } from "@tauri-apps/plugin-dialog";
import { api } from "@/api";
import { GitStatusPanel } from "@/components/GitStatus/GitStatusPanel";
import { TaskList } from "@/components/TaskList/TaskList";
import { ProjectCard } from "@/components/ProjectCard/ProjectCard";
import { useProjectStore } from "@/stores";
import { useTranslation, useLocaleStore } from "@/i18n";
import type { ProjectDetail, Task } from "@/types";
import type { GitCommitInfo, GitStatus } from "@/types";
import { LANGUAGE_ICONS } from "@/types";

export function ProjectDetailPage() {
  const { t } = useTranslation();
  const locale = useLocaleStore((s) => s.locale);
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const {
    removeProject,
    openProjectFolder,
    openInEditor,
    markOpened,
    updateProject,
    projects,
    fetchProjects,
    getChildProjects,
  } = useProjectStore();
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [groupName, setGroupName] = useState("");
  const [savingGroup, setSavingGroup] = useState(false);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [commits, setCommits] = useState<GitCommitInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);

  useEffect(() => {
    fetchProjects();
  }, [fetchProjects]);

  useEffect(() => {
    if (!id) return;
    markOpened(id);
    loadProject(id);
  }, [id]);

  const loadProject = async (projectId: string) => {
    setLoading(true);
    try {
      const [d, taskList] = await Promise.all([
        api.projects.getDetail(projectId),
        api.tasks.listByProject(projectId),
      ]);
      setDetail(d);
      setGroupName(d.project.group_name ?? "");
      setTasks(taskList);

      try {
        const git = await api.git.status(projectId);
        setGitStatus(git.status);
        setCommits(git.recent_commits);
      } catch {
        setGitStatus(null);
        setCommits([]);
      }
    } finally {
      setLoading(false);
    }
  };

  const runCommand = async (command: string) => {
    if (!id) return;
    setRunning(command);
    setOutput(null);
    try {
      const result = await api.commands.run(id, command);
      setOutput(
        result.stdout || result.stderr || `Exit code: ${result.exit_code}`,
      );
    } catch (e) {
      setOutput(String(e));
    } finally {
      setRunning(null);
    }
  };

  const handleOpenFolder = useCallback(async () => {
    if (!id) return;
    await openProjectFolder(id);
  }, [id, openProjectFolder]);

  const handleOpenEditor = useCallback(async () => {
    if (!id) return;
    await openInEditor(id);
  }, [id, openInEditor]);

  const handleSaveGroup = useCallback(async () => {
    if (!id || !detail) return;
    const trimmed = groupName.trim();
    if (trimmed === (detail.project.group_name ?? "")) return;

    setSavingGroup(true);
    try {
      const updated = await updateProject({
        id,
        group_name: trimmed || null,
      });
      setDetail((prev) =>
        prev ? { ...prev, project: { ...prev.project, ...updated } } : prev,
      );
    } finally {
      setSavingGroup(false);
    }
  }, [id, groupName, detail, updateProject]);

  const handleArchive = useCallback(async () => {
    if (!detail) return;
    const newStatus =
      detail.project.status === "archived" ? "active" : "archived";
    const updated = await updateProject({
      id: detail.project.id,
      status: newStatus,
    });
    setDetail((prev) =>
      prev ? { ...prev, project: { ...prev.project, ...updated } } : prev,
    );
  }, [detail, updateProject]);

  const handleRemove = useCallback(async () => {
    if (!detail) return;
    const confirmed = await ask(
      t("projects.removeMessage", { name: detail.project.name }),
      {
        title: t("projects.removeTitle"),
        kind: "warning",
        okLabel: t("common.remove"),
        cancelLabel: t("common.cancel"),
      },
    );
    if (confirmed) {
      await removeProject(detail.project.id);
      navigate("/projects");
    }
  }, [detail, removeProject, navigate, t]);

  if (loading) return <div className="page">{t("projectDetail.loading")}</div>;
  if (!detail) return <div className="page">{t("projectDetail.notFound")}</div>;

  const { project, packages, commands } = detail;
  const listItem = projects.find((p) => p.id === project.id);
  const childProjects = id ? getChildProjects(id) : [];
  const parentProject = listItem?.parent_id
    ? projects.find((p) => p.id === listItem.parent_id)
    : null;
  const icon =
    project.icon ??
    (childProjects.length > 0 ? "📁" : LANGUAGE_ICONS[project.language]) ??
    "📁";

  return (
    <div className="page project-detail">
      <Link to="/projects" className="back-link">
        {t("common.backToProjects")}
      </Link>

      {parentProject && (
        <Link to={`/projects/${parentProject.id}`} className="back-link parent-link">
          ↑ {parentProject.name}
        </Link>
      )}

      <header className="project-detail-header panel">
        <span className="project-icon-large">{icon}</span>
        <div className="project-detail-info">
          <h1>{project.name.toUpperCase()}</h1>
          <p className="muted">
            {t(`enums.language.${project.language}`)} ·{" "}
            {t(`enums.projectType.${project.project_type}`)}
            {project.last_modified_at && (
              <>
                {" · "}
                {t("projectDetail.lastModified")}:{" "}
                {formatDate(project.last_modified_at, locale)}
              </>
            )}
          </p>
          <code className="path">{project.root_path}</code>

          <div className="project-group-field">
            <label htmlFor="project-group">{t("projectDetail.group")}</label>
            <div className="inline-field">
              <input
                id="project-group"
                type="text"
                value={groupName}
                placeholder={t("projectDetail.groupPlaceholder")}
                onChange={(e) => setGroupName(e.target.value)}
                onBlur={handleSaveGroup}
                disabled={savingGroup}
              />
            </div>
          </div>
        </div>
        <div className="project-detail-actions">
          <button type="button" className="btn accent" onClick={handleOpenEditor}>
            ⌘ {t("projectDetail.openInEditor")}
          </button>
          <button type="button" className="btn" onClick={handleOpenFolder}>
            📂 {t("common.openFolder")}
          </button>
          <button type="button" className="btn" onClick={handleArchive}>
            📦{" "}
            {project.status === "archived"
              ? t("projectDetail.unarchive")
              : t("projectDetail.archive")}
          </button>
          <button type="button" className="btn danger" onClick={handleRemove}>
            {t("projectDetail.remove")}
          </button>
        </div>
      </header>

      {childProjects.length > 0 && (
        <section className="panel subprojects-section">
          <h2>{t("projectDetail.subprojects")}</h2>
          <p className="muted">{t("projectDetail.subprojectsHint")}</p>
          <div className="subprojects-grid">
            {childProjects.map((child) => (
              <ProjectCard
                key={child.id}
                project={child}
                compact
                onOpenFolder={async (p) => openProjectFolder(p.id)}
                onOpenEditor={async (p) => openInEditor(p.id)}
                onNavigate={async (p) => {
                  await markOpened(p.id);
                  navigate(`/projects/${p.id}`);
                }}
                onRemove={async (p) => {
                  const confirmed = await ask(
                    t("projects.removeMessage", { name: p.name }),
                    {
                      title: t("projects.removeTitle"),
                      kind: "warning",
                      okLabel: t("common.remove"),
                      cancelLabel: t("common.cancel"),
                    },
                  );
                  if (confirmed) {
                    await removeProject(p.id);
                    await fetchProjects();
                  }
                }}
                onArchive={async (p) => {
                  await updateProject({
                    id: p.id,
                    status: p.status === "archived" ? "active" : "archived",
                  });
                  await fetchProjects();
                }}
              />
            ))}
          </div>
        </section>
      )}

      <div className="detail-grid">
        <section className="panel">
          <h2>{t("projectDetail.git")}</h2>
          <GitStatusPanel status={gitStatus} recentCommits={commits} />
        </section>

        <section className="panel">
          <h2>{t("projectDetail.packages")}</h2>
          {packages.length > 0 ? (
            <ul className="simple-list">
              {packages.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          ) : (
            <p className="muted">{t("projectDetail.noPackages")}</p>
          )}
        </section>

        <section className="panel">
          <h2>{t("projectDetail.commands")}</h2>
          <div className="command-buttons">
            {commands.map((c) => (
              <button
                key={c.command}
                className="btn"
                disabled={running === c.command}
                onClick={() => runCommand(c.command)}
              >
                ▶ {c.name}
              </button>
            ))}
          </div>
          {output && <pre className="command-output">{output}</pre>}
        </section>

        <section className="panel">
          <h2>{t("projectDetail.tasks")}</h2>
          <TaskList
            tasks={tasks}
            onToggle={async (task) => {
              const updated = await api.tasks.update({
                id: task.id,
                status: task.status === "done" ? "todo" : "done",
              });
              setTasks((prev) =>
                prev.map((item) => (item.id === updated.id ? updated : item)),
              );
            }}
          />
        </section>
      </div>
    </div>
  );
}

function formatDate(iso: string, locale: string): string {
  return new Date(iso).toLocaleDateString(locale, {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
