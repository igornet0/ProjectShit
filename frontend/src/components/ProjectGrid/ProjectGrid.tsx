import type { ReactNode } from "react";
import type { ProjectListItem } from "@/types";
import type { VisibleProjectRow } from "@/utils/projectTree";
import { ProjectCard } from "@/components/ProjectCard/ProjectCard";
import { Icon } from "@/components/Icon/Icon";
import { useTranslation } from "@/i18n";

interface ProjectGridProps {
  rows: VisibleProjectRow[];
  loading?: boolean;
  expandedIds: Set<string>;
  onToggleExpand: (project: ProjectListItem) => void;
  onOpenFolder: (project: ProjectListItem) => void;
  onOpenEditor: (project: ProjectListItem) => void;
  onNavigate: (project: ProjectListItem) => void;
  onRemove: (project: ProjectListItem) => void;
  onArchive: (project: ProjectListItem) => void;
}

export function ProjectGrid({
  rows,
  loading,
  expandedIds,
  onToggleExpand,
  onOpenFolder,
  onOpenEditor,
  onNavigate,
  onRemove,
  onArchive,
}: ProjectGridProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="panel empty-state">
        <p>{t("projects.loading")}</p>
      </div>
    );
  }

  if (rows.length === 0) {
    return (
      <div className="panel empty-state">
        <Icon name="folder" size={28} />
        <p className="empty-state-title">{t("projects.emptyTitle")}</p>
        <p>{t("projects.emptyHint")}</p>
      </div>
    );
  }

  return (
    <ProjectTable>
      {rows.map(({ project, depth }) => (
        <ProjectCard
          key={project.id}
          project={project}
          depth={depth}
          expanded={expandedIds.has(project.id)}
          onToggleExpand={onToggleExpand}
          onOpenFolder={onOpenFolder}
          onOpenEditor={onOpenEditor}
          onNavigate={onNavigate}
          onRemove={onRemove}
          onArchive={onArchive}
        />
      ))}
    </ProjectTable>
  );
}

/** Table frame with column headers shared by local and compact project rows. */
export function ProjectTable({
  children,
  compact = false,
}: {
  children: ReactNode;
  compact?: boolean;
}) {
  const { t } = useTranslation();

  return (
    <div className={`panel project-table${compact ? " compact" : ""}`} role="table">
      <div className="project-table-head" role="row">
        <span role="columnheader">{t("projects.columns.name")}</span>
        {!compact && (
          <span className="col-path" role="columnheader">
            {t("projects.columns.path")}
          </span>
        )}
        <span className="col-lang" role="columnheader">
          {t("projects.filterLanguage")}
        </span>
        {!compact && (
          <span className="col-modified" role="columnheader">
            {t("projects.lastModified")}
          </span>
        )}
        {!compact && (
          <span className="col-folder" role="columnheader">
            {t("folders.title")}
          </span>
        )}
        <span role="columnheader" />
      </div>
      {children}
    </div>
  );
}
