import { useEffect, useCallback, useMemo, useState } from "react";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { ProjectGrid } from "@/components/ProjectGrid/ProjectGrid";
import { FolderSidebar } from "@/components/FolderSidebar/FolderSidebar";
import { GhostProjectCard } from "@/components/GhostProjectCard/GhostProjectCard";
import { useProjectStore, useFolderStore, useGitHubStore } from "@/stores";
import { useTranslation } from "@/i18n";
import type { HubProjectEntry, ProjectListItem } from "@/types";

const LANGUAGES = [
  "all",
  "rust",
  "python",
  "javascript",
  "typescript",
  "go",
  "java",
  "cpp",
  "unknown",
] as const;

export function ProjectsPage() {
  const { t } = useTranslation();
  const {
    projects,
    loading,
    error,
    searchQuery,
    filterGroup,
    filterLanguage,
    filterFolder,
    showArchived,
    showGithub,
    groups,
    fetchProjects,
    fetchGroups,
    addRoot,
    removeProject,
    openProjectFolder,
    openInEditor,
    markOpened,
    updateProject,
    setSearchQuery,
    setFilterGroup,
    setFilterLanguage,
    setShowArchived,
    setShowGithub,
    toggleProjectExpanded,
    visibleProjectRows,
    expandedProjectIds,
  } = useProjectStore();

  const { fetchFolders } = useFolderStore();
  const { config, hubEntries, fetchConfig, fetchHubEntries, cloneRepo } =
    useGitHubStore();
  const [cloning, setCloning] = useState<string | null>(null);

  const rows = useMemo(
    () => visibleProjectRows(),
    [
      visibleProjectRows,
      projects,
      searchQuery,
      filterGroup,
      filterLanguage,
      filterFolder,
      loading,
      expandedProjectIds,
    ],
  );

  const ghosts = useMemo(
    () => hubEntries.filter((e) => e.kind === "ghost"),
    [hubEntries],
  );

  const expandedIds = useMemo(
    () => new Set(expandedProjectIds),
    [expandedProjectIds],
  );

  useEffect(() => {
    fetchProjects();
    fetchGroups();
    fetchFolders();
    fetchConfig();
  }, [fetchProjects, fetchGroups, fetchFolders, fetchConfig]);

  useEffect(() => {
    fetchProjects();
  }, [showArchived, fetchProjects]);

  useEffect(() => {
    if (config?.connected && showGithub) {
      fetchHubEntries();
    }
  }, [config?.connected, showGithub, fetchHubEntries]);

  const handleAddProject = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("projects.selectDirectory"),
    });
    if (selected && typeof selected === "string") {
      await addRoot(selected);
      await fetchFolders();
    }
  };

  const handleOpenFolder = useCallback(
    async (project: ProjectListItem) => {
      await openProjectFolder(project.id);
    },
    [openProjectFolder],
  );

  const handleOpenEditor = useCallback(
    async (project: ProjectListItem) => {
      await openInEditor(project.id);
    },
    [openInEditor],
  );

  const handleNavigate = useCallback(
    async (project: ProjectListItem) => {
      await markOpened(project.id);
    },
    [markOpened],
  );

  const handleRemove = useCallback(
    async (project: ProjectListItem) => {
      const confirmed = await ask(
        t("projects.removeMessage", { name: project.name }),
        {
          title: t("projects.removeTitle"),
          kind: "warning",
          okLabel: t("common.remove"),
          cancelLabel: t("common.cancel"),
        },
      );
      if (confirmed) {
        await removeProject(project.id);
        await fetchFolders();
      }
    },
    [removeProject, fetchFolders, t],
  );

  const handleToggleExpand = useCallback(
    (project: ProjectListItem) => {
      toggleProjectExpanded(project.id);
    },
    [toggleProjectExpanded],
  );

  const handleArchive = useCallback(
    async (project: ProjectListItem) => {
      const newStatus = project.status === "archived" ? "active" : "archived";
      await updateProject({ id: project.id, status: newStatus });
      if (!showArchived && newStatus === "archived") {
        await fetchProjects();
      }
    },
    [updateProject, showArchived, fetchProjects],
  );

  const handleClone = useCallback(
    async (entry: HubProjectEntry) => {
      setCloning(entry.github_repo.full_name);
      try {
        await cloneRepo(entry.github_repo.full_name);
        await fetchFolders();
      } finally {
        setCloning(null);
      }
    },
    [cloneRepo, fetchFolders],
  );

  const groupOptions = useMemo(
    () => ["all", "ungrouped", ...groups],
    [groups],
  );

  return (
    <div className="page projects">
      <header className="page-header row hero-header">
        <div>
          <p className="eyebrow">{t("nav.projects")}</p>
          <h1>{t("projects.title")}</h1>
          <p className="muted">{t("projects.subtitle")}</p>
        </div>
        <button className="btn primary" onClick={handleAddProject}>
          {t("projects.addProject")}
        </button>
      </header>

      <div className="projects-layout">
        <FolderSidebar />

        <div className="projects-main">
          <div className="projects-toolbar panel">
            <div className="search-bar inline">
              <input
                type="search"
                placeholder={t("projects.searchPlaceholder")}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>

            <div className="filter-row">
              <label className="filter-field">
                <span>{t("projects.filterGroup")}</span>
                <select
                  value={filterGroup}
                  onChange={(e) => setFilterGroup(e.target.value)}
                >
                  {groupOptions.map((g) => (
                    <option key={g} value={g}>
                      {g === "all"
                        ? t("projects.allGroups")
                        : g === "ungrouped"
                          ? t("projects.ungrouped")
                          : g}
                    </option>
                  ))}
                </select>
              </label>

              <label className="filter-field">
                <span>{t("projects.filterLanguage")}</span>
                <select
                  value={filterLanguage}
                  onChange={(e) => setFilterLanguage(e.target.value)}
                >
                  {LANGUAGES.map((lang) => (
                    <option key={lang} value={lang}>
                      {lang === "all"
                        ? t("projects.allLanguages")
                        : t(`enums.language.${lang}`)}
                    </option>
                  ))}
                </select>
              </label>

              <label className="filter-toggle">
                <input
                  type="checkbox"
                  checked={showArchived}
                  onChange={(e) => setShowArchived(e.target.checked)}
                />
                <span>{t("projects.showArchived")}</span>
              </label>

              {config?.connected && (
                <label className="filter-toggle">
                  <input
                    type="checkbox"
                    checked={showGithub}
                    onChange={(e) => setShowGithub(e.target.checked)}
                  />
                  <span>{t("projects.showGithub")}</span>
                </label>
              )}
            </div>
          </div>

          {error && <div className="error-banner">{error}</div>}

          <ProjectGrid
            rows={rows}
            loading={loading}
            expandedIds={expandedIds}
            onToggleExpand={handleToggleExpand}
            onOpenFolder={handleOpenFolder}
            onOpenEditor={handleOpenEditor}
            onNavigate={handleNavigate}
            onRemove={handleRemove}
            onArchive={handleArchive}
          />

          {showGithub && config?.connected && ghosts.length > 0 && (
            <section className="ghost-section">
              <h2>{t("projects.ghostSection")}</h2>
              <p className="muted">{t("projects.ghostHint")}</p>
              <div className="project-grid">
                {ghosts.map((entry) => (
                  <GhostProjectCard
                    key={entry.github_repo.id}
                    entry={entry}
                    cloning={cloning === entry.github_repo.full_name}
                    onClone={handleClone}
                  />
                ))}
              </div>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}
