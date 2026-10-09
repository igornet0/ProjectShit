import { useEffect, useState, useCallback } from "react";
import { useParams, Link, useNavigate } from "react-router-dom";
import { ask } from "@tauri-apps/plugin-dialog";
import { api } from "@/api";
import { GitStatusPanel } from "@/components/GitStatus/GitStatusPanel";
import { TaskList } from "@/components/TaskList/TaskList";
import { BrddPanel } from "@/components/BrddPanel/BrddPanel";
import { ProjectCard } from "@/components/ProjectCard/ProjectCard";
import { ProjectTable } from "@/components/ProjectGrid/ProjectGrid";
import { Icon } from "@/components/Icon/Icon";
import { useProjectStore } from "@/stores";
import { useTranslation, useLocaleStore } from "@/i18n";
import type { ProjectDetail, Task } from "@/types";
import type { GitCommitInfo, GitStatus } from "@/types";
import { formatDate, formatRelative, languageColor } from "@/utils/format";

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
      // Git status is the slowest call; start it alongside the others.
      const gitRequest = api.git.status(projectId).catch(() => null);
      const [d, taskList] = await Promise.all([
        api.projects.getDetail(projectId),
        api.tasks.listByProject(projectId),
      ]);
      setDetail(d);
      setGroupName(d.project.group_name ?? "");
      setTasks(taskList);

      const git = await gitRequest;
      setGitStatus(git?.status ?? null);
      setCommits(git?.recent_commits ?? []);
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

  if (loading) return <div className="page page-state">{t("projectDetail.loading")}</div>;
  if (!detail) return <div className="page page-state">{t("projectDetail.notFound")}</div>;

  const { project, packages, commands } = detail;
  const listItem = projects.find((p) => p.id === project.id);
  const childProjects = id ? getChildProjects(id) : [];
  const parentProject = listItem?.parent_id
    ? projects.find((p) => p.id === listItem.parent_id)
    : null;
  const archived = project.status === "archived";

  const confirmRemove = async (name: string) =>
    ask(t("projects.removeMessage", { name }), {
      title: t("projects.removeTitle"),
      kind: "warning",
      okLabel: t("common.remove"),
      cancelLabel: t("common.cancel"),
    });

  return (
    <div className="page project-detail">
      <nav className="breadcrumb" aria-label="breadcrumb">
        <Link to="/projects">{t("nav.projects")}</Link>
        {parentProject && (
          <>
            <Icon name="chevronRight" size={12} />
            <Link to={`/projects/${parentProject.id}`}>{parentProject.name}</Link>
          </>
        )}
        <Icon name="chevronRight" size={12} />
        <span aria-current="page">{project.name}</span>
      </nav>

      <header className="detail-header">
        <span
          className="avatar large"
          style={{ "--lang-color": languageColor(project.language) } as React.CSSProperties}
          aria-hidden="true"
        >
          {project.icon ?? project.name.charAt(0).toUpperCase()}
        </span>

        <div className="detail-title">
          <h1>{project.name}</h1>
          <div className="detail-meta">
            <span className="lang">
              <span
                className="dot"
                style={{ "--dot-color": languageColor(project.language) } as React.CSSProperties}
              />
              {t(`enums.language.${project.language}`)}
            </span>
            <span className="detail-meta-item">
              <Icon name="package" size={13} />
              {t(`enums.projectType.${project.project_type}`)}
            </span>
            {gitStatus && (
              <span className="detail-meta-item">
                <Icon name="branch" size={13} />
                <span className="mono">{gitStatus.branch}</span>
              </span>
            )}
            {project.last_modified_at && (
              <span
                className="detail-meta-item"
                title={formatDate(project.last_modified_at, locale, DATE_TIME)}
              >
                <Icon name="clock" size={13} />
                {formatRelative(project.last_modified_at, locale)}
              </span>
            )}
            {archived && <span className="badge">{t("enums.status.archived")}</span>}
          </div>
          <div className="detail-path">
            <code title={project.root_path}>{project.root_path}</code>
            <button
              type="button"
              className="icon-btn small"
              title={t("settings.githubOAuthCodeCopy")}
              aria-label={t("settings.githubOAuthCodeCopy")}
              onClick={() => void navigator.clipboard?.writeText(project.root_path)}
            >
              <Icon name="copy" size={12} />
            </button>
          </div>
          <div className="group-field">
            <label htmlFor="project-group">{t("projectDetail.group")}</label>
            <input
              id="project-group"
              type="text"
              value={groupName}
              placeholder={t("projectDetail.groupPlaceholder")}
              onChange={(e) => setGroupName(e.target.value)}
              onBlur={handleSaveGroup}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
              }}
              disabled={savingGroup}
            />
          </div>
        </div>

        <div className="detail-actions">
          <button type="button" className="btn primary" onClick={handleOpenEditor}>
            <Icon name="code" size={14} />
            {t("projectDetail.openInEditor")}
          </button>
          <button type="button" className="btn" onClick={handleOpenFolder}>
            <Icon name="external" size={14} />
            {t("common.openFolder")}
          </button>
          <button
            type="button"
            className="btn ghost"
            onClick={handleArchive}
            title={archived ? t("projectDetail.unarchive") : t("projectDetail.archive")}
          >
            <Icon name="archive" size={14} />
            {archived ? t("projectDetail.unarchive") : t("projectDetail.archive")}
          </button>
          <button
            type="button"
            className="icon-btn danger"
            onClick={handleRemove}
            title={t("projectDetail.remove")}
            aria-label={t("projectDetail.remove")}
          >
            <Icon name="trash" size={15} />
          </button>
        </div>
      </header>

      <div className="detail-grid">
        <section className="panel">
          <div className="panel-header">
            <h2>
              <Icon name="branch" size={14} />
              {t("projectDetail.git")}
            </h2>
          </div>
          <GitStatusPanel status={gitStatus} recentCommits={commits} />
        </section>

        <section className="panel">
          <div className="panel-header">
            <h2>
              <Icon name="terminal" size={14} />
              {t("projectDetail.commands")}
            </h2>
          </div>
          {commands.length === 0 ? (
            <p className="panel-empty">—</p>
          ) : (
            <div>
              {commands.map((c) => (
                <div key={c.command} className="command-row">
                  <span className="command-row-name">{c.name}</span>
                  <span className="command-row-cmd" title={c.command}>
                    {c.command}
                  </span>
                  <button
                    type="button"
                    className="icon-btn"
                    disabled={running !== null}
                    title={c.command}
                    aria-label={c.name}
                    onClick={() => runCommand(c.command)}
                  >
                    <Icon name={running === c.command ? "refresh" : "play"} size={14} />
                  </button>
                </div>
              ))}
            </div>
          )}
          {output && (
            <div className="terminal">
              <div className="terminal-head">
                <span>output</span>
                <button
                  type="button"
                  className="icon-btn small"
                  aria-label={t("commandPalette.close")}
                  onClick={() => setOutput(null)}
                >
                  <Icon name="x" size={12} />
                </button>
              </div>
              <pre>{output}</pre>
            </div>
          )}
        </section>

        {childProjects.length > 0 && (
          <section className="span-2">
            <p className="section-label">
              {t("projectDetail.subprojects")} · {childProjects.length}
            </p>
            <ProjectTable compact>
              {childProjects.map((child) => (
                <ProjectCard
                  key={child.id}
                  project={child}
                  compact
                  onOpenFolder={async (p) => openProjectFolder(p.id)}
                  onOpenEditor={async (p) => openInEditor(p.id)}
                  onNavigate={async (p) => {
                    await markOpened(p.id);
                  }}
                  onRemove={async (p) => {
                    if (await confirmRemove(p.name)) {
                      await removeProject(p.id);
                      await fetchProjects(true);
                    }
                  }}
                  onArchive={async (p) => {
                    await updateProject({
                      id: p.id,
                      status: p.status === "archived" ? "active" : "archived",
                    });
                    await fetchProjects(true);
                  }}
                />
              ))}
            </ProjectTable>
          </section>
        )}

        <section className="panel">
          <div className="panel-header">
            <h2>
              <Icon name="tasks" size={14} />
              {t("projectDetail.tasks")}
            </h2>
          </div>
          <TaskList
            tasks={tasks}
            onToggle={async (task) => {
              const updated = await api.tasks.update({
                id: task.id,
                status: task.status === "done" ? "todo" : "done",
              });
              if (task.recurrence && updated.status === "done") {
                // The backend created the next occurrence of the series.
                setTasks(await api.tasks.listByProject(project.id));
                return;
              }
              setTasks((prev) =>
                prev.map((item) => (item.id === updated.id ? updated : item)),
              );
            }}
          />
        </section>

        <BrddPanel projectId={project.id} />

        <section className="panel">
          <div className="panel-header">
            <h2>
              <Icon name="package" size={14} />
              {t("projectDetail.packages")}
            </h2>
          </div>
          {packages.length > 0 ? (
            <div className="package-list">
              {packages.map((p) => (
                <span key={p} className="badge">
                  {p}
                </span>
              ))}
            </div>
          ) : (
            <p className="panel-empty">{t("projectDetail.noPackages")}</p>
          )}
        </section>
      </div>
    </div>
  );
}

const DATE_TIME: Intl.DateTimeFormatOptions = {
  month: "short",
  day: "numeric",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
};
