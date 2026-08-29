import { Link } from "react-router-dom";
import type { ProjectListItem } from "@/types";
import { LANGUAGE_ICONS, STATUS_COLORS } from "@/types";
import { useTranslation, useLocaleStore } from "@/i18n";
import { useFolderStore, useProjectStore } from "@/stores";

interface ProjectCardProps {
  project: ProjectListItem;
  depth?: number;
  expanded?: boolean;
  compact?: boolean;
  onToggleExpand?: (project: ProjectListItem) => void;
  onOpenFolder: (project: ProjectListItem) => void;
  onOpenEditor: (project: ProjectListItem) => void;
  onNavigate: (project: ProjectListItem) => void;
  onRemove: (project: ProjectListItem) => void;
  onArchive: (project: ProjectListItem) => void;
}

export function ProjectCard({
  project,
  depth = 0,
  expanded = false,
  compact = false,
  onToggleExpand,
  onOpenFolder,
  onOpenEditor,
  onNavigate,
  onRemove,
  onArchive,
}: ProjectCardProps) {
  const { t } = useTranslation();
  const locale = useLocaleStore((s) => s.locale);
  const { folders } = useFolderStore();
  const { assignToFolder } = useProjectStore();
  const hasChildren = project.child_count > 0;
  const icon =
    project.icon ??
    (hasChildren ? "📁" : LANGUAGE_ICONS[project.language]) ??
    "📁";
  const statusColor = STATUS_COLORS[project.status];

  const handleAction =
    (action: (project: ProjectListItem) => void) =>
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      action(project);
    };

  return (
    <article
      className={`project-card ${project.status === "archived" ? "archived" : ""} ${hasChildren ? "has-children" : ""} ${compact ? "compact" : ""}`}
      data-lang={project.language}
      style={{ "--tree-depth": depth } as React.CSSProperties}
    >
      {project.git_dirty && (
        <span className="git-warning" title={t("projects.gitDirty")}>
          ⚠️
        </span>
      )}

      {hasChildren && onToggleExpand && (
        <button
          type="button"
          className="project-expand-btn"
          title={
            expanded ? t("projects.collapseChildren") : t("projects.expandChildren")
          }
          aria-label={
            expanded ? t("projects.collapseChildren") : t("projects.expandChildren")
          }
          aria-expanded={expanded}
          onClick={handleAction(onToggleExpand)}
        >
          {expanded ? "▾" : "▸"}
        </button>
      )}

      <Link
        to={`/projects/${project.id}`}
        className="project-card-link"
        onClick={() => onNavigate(project)}
      >
        <div className="project-card-header">
          <div className="project-card-icon">{icon}</div>
          <div className="project-card-badges">
            {project.group_name && (
              <span className="project-group-badge">{project.group_name}</span>
            )}
            {hasChildren && (
              <span className="project-child-count">
                {t("projects.childCount", { count: project.child_count })}
              </span>
            )}
          </div>
        </div>

        <h3 className="project-card-name">{project.name}</h3>

        {!compact && (
          <div className="project-card-meta">
            <span className="project-meta-chip">
              {t(`enums.language.${project.language}`)}
            </span>
            <span className="project-meta-chip project-meta-status">
              <span className="status-dot" style={{ background: statusColor }} />
              {t(`enums.status.${project.status}`)}
            </span>
            <span className="project-meta-chip muted">
              {t(`enums.projectType.${project.project_type}`)}
            </span>
          </div>
        )}

        {project.last_modified_at && !compact && (
          <p className="project-card-modified">
            {t("projects.lastModified")}: {formatDate(project.last_modified_at, locale)}
          </p>
        )}

        {!compact && (
          <p className="project-card-path muted">{shortPath(project.root_path)}</p>
        )}
      </Link>

      {!compact && (
        <footer className="project-card-footer">
          {folders.length > 0 && (
            <div
              className="project-folder-picker"
              onClick={(e) => e.stopPropagation()}
              onMouseDown={(e) => e.stopPropagation()}
            >
              <span className="project-folder-picker-icon">🗂️</span>
              <label className="sr-only" htmlFor={`folder-${project.id}`}>
                {t("folders.assign")}
              </label>
              <select
                id={`folder-${project.id}`}
                className="project-folder-select"
                value={String(project.folder_id ?? "")}
                onChange={(e) => {
                  e.stopPropagation();
                  void assignToFolder(project.id, e.target.value || null);
                }}
              >
                <option value="">{t("folders.unassigned")}</option>
                {folders.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div className="project-card-actions">
            <button
              type="button"
              className="icon-btn accent"
              title={t("projects.openInEditor")}
              aria-label={t("projects.openInEditor")}
              onClick={handleAction(onOpenEditor)}
            >
              ⌘
            </button>
            <button
              type="button"
              className="icon-btn"
              title={t("projects.openFolder")}
              aria-label={t("projects.openProject", { name: project.name })}
              onClick={handleAction(onOpenFolder)}
            >
              📂
            </button>
            <button
              type="button"
              className="icon-btn"
              title={
                project.status === "archived"
                  ? t("projects.unarchive")
                  : t("projects.archive")
              }
              aria-label={
                project.status === "archived"
                  ? t("projects.unarchive")
                  : t("projects.archive")
              }
              onClick={handleAction(onArchive)}
            >
              📦
            </button>
            <button
              type="button"
              className="icon-btn danger"
              title={t("projects.removeFromHub")}
              aria-label={t("common.remove")}
              onClick={handleAction(onRemove)}
            >
              ✕
            </button>
          </div>
        </footer>
      )}
    </article>
  );
}

function formatDate(iso: string, locale: string): string {
  return new Date(iso).toLocaleDateString(locale, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function shortPath(path: string): string {
  const home = path.replace(/^\/Users\/[^/]+/, "~");
  return home.length > 48 ? `…${home.slice(-45)}` : home;
}
