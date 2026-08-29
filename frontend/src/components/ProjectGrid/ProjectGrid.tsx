import type { ProjectListItem } from "@/types";
import type { VisibleProjectRow } from "@/utils/projectTree";
import { ProjectCard } from "@/components/ProjectCard/ProjectCard";
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
    return <div className="empty-state">{t("projects.loading")}</div>;
  }

  if (rows.length === 0) {
    return (
      <div className="empty-state">
        <p>{t("projects.emptyTitle")}</p>
        <p className="muted">{t("projects.emptyHint")}</p>
      </div>
    );
  }

  return (
    <div className="project-grid project-grid--tree">
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
    </div>
  );
}
