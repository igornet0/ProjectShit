import { Link } from "react-router-dom";
import type { ProjectListItem } from "@/types";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";
import { useFolderStore, useProjectStore } from "@/stores";
import { formatRelative, languageColor, shortPath } from "@/utils/format";

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

/** One project rendered as a row of the project table. */
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
  const { t, locale } = useTranslation();
  const folders = useFolderStore((s) => s.folders);
  const assignToFolder = useProjectStore((s) => s.assignToFolder);
  const hasChildren = project.child_count > 0;
  const archived = project.status === "archived";
  const archiveLabel = archived ? t("projects.unarchive") : t("projects.archive");

  const handleAction =
    (action: (project: ProjectListItem) => void) =>
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      action(project);
    };

  const lang = (
    <span className="lang project-lang">
      <span
        className="dot"
        style={{ "--dot-color": languageColor(project.language) } as React.CSSProperties}
      />
      {t(`enums.language.${project.language}`)}
    </span>
  );

  return (
    <div
      className={`project-row${archived ? " archived" : ""}${compact ? " compact" : ""}`}
      role="row"
      style={{ "--tree-depth": depth } as React.CSSProperties}
    >
      <div className="project-name-cell" role="cell">
        {!compact &&
          (hasChildren && onToggleExpand ? (
            <button
              type="button"
              className="tree-toggle"
              aria-label={
                expanded ? t("projects.collapseChildren") : t("projects.expandChildren")
              }
              title={expanded ? t("projects.collapseChildren") : t("projects.expandChildren")}
              aria-expanded={expanded}
              onClick={handleAction(onToggleExpand)}
            >
              <Icon name={expanded ? "chevronDown" : "chevronRight"} size={14} />
            </button>
          ) : (
            <span className="tree-spacer" />
          ))}

        {project.icon && <span aria-hidden="true">{project.icon}</span>}

        <Link
          to={`/projects/${project.id}`}
          className="project-name"
          title={project.root_path}
          onClick={() => onNavigate(project)}
        >
          {project.name}
        </Link>

        {project.git_dirty && (
          <span className="dirty-dot" title={t("projects.gitDirty")} />
        )}
        {project.group_name && <span className="badge">{project.group_name}</span>}
        {hasChildren && (
          <span className="badge outline">
            {t("projects.childCount", { count: project.child_count })}
          </span>
        )}
        {archived && <span className="badge">{t("enums.status.archived")}</span>}
      </div>

      {!compact && (
        <span className="project-path" role="cell" title={project.root_path}>
          {shortPath(project.root_path, 34)}
        </span>
      )}

      <span role="cell">{lang}</span>

      {!compact && (
        <span
          className="project-modified"
          role="cell"
          title={project.last_modified_at ?? undefined}
        >
          {project.last_modified_at ? formatRelative(project.last_modified_at, locale) : "—"}
        </span>
      )}

      {!compact && (
        <span className="project-folder" role="cell">
          {folders.length > 0 && (
            <>
              <label className="sr-only" htmlFor={`folder-${project.id}`}>
                {t("folders.assign")}
              </label>
              <select
                id={`folder-${project.id}`}
                className={project.folder_id ? undefined : "unassigned"}
                value={String(project.folder_id ?? "")}
                title={t("folders.assign")}
                onChange={(e) => void assignToFolder(project.id, e.target.value || null)}
              >
                <option value="">{t("folders.unassigned")}</option>
                {folders.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.name}
                  </option>
                ))}
              </select>
            </>
          )}
        </span>
      )}

      <div className="project-actions" role="cell">
        <button
          type="button"
          className="icon-btn"
          title={t("projects.openInEditor")}
          aria-label={t("projects.openInEditor")}
          onClick={handleAction(onOpenEditor)}
        >
          <Icon name="code" size={15} />
        </button>
        <button
          type="button"
          className="icon-btn"
          title={t("projects.openFolder")}
          aria-label={t("projects.openProject", { name: project.name })}
          onClick={handleAction(onOpenFolder)}
        >
          <Icon name="external" size={15} />
        </button>
        <button
          type="button"
          className="icon-btn"
          title={archiveLabel}
          aria-label={archiveLabel}
          onClick={handleAction(onArchive)}
        >
          <Icon name="archive" size={15} />
        </button>
        <button
          type="button"
          className="icon-btn danger"
          title={t("projects.removeFromHub")}
          aria-label={t("common.remove")}
          onClick={handleAction(onRemove)}
        >
          <Icon name="trash" size={15} />
        </button>
      </div>
    </div>
  );
}
