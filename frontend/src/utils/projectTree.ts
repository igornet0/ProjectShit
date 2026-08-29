import type { ProjectListItem } from "@/types";

export interface VisibleProjectRow {
  project: ProjectListItem;
  depth: number;
}

function passesFilters(
  project: ProjectListItem,
  searchQuery: string,
  filterGroup: string,
  filterLanguage: string,
  filterFolder: string,
): boolean {
  const q = searchQuery.trim().toLowerCase();
  if (
    q &&
    !project.name.toLowerCase().includes(q) &&
    !project.root_path.toLowerCase().includes(q)
  ) {
    return false;
  }
  if (filterGroup !== "all") {
    if (filterGroup === "ungrouped") {
      if (project.group_name) return false;
    } else if (project.group_name !== filterGroup) {
      return false;
    }
  }
  if (filterFolder !== "all") {
    if (filterFolder === "none") {
      if (project.folder_id) return false;
    } else if (String(project.folder_id ?? "") !== filterFolder) {
      return false;
    }
  }
  if (filterLanguage !== "all" && project.language !== filterLanguage) {
    return false;
  }
  return true;
}

function sortByRecency(a: ProjectListItem, b: ProjectListItem): number {
  const aOpened = a.last_opened_at ? new Date(a.last_opened_at).getTime() : 0;
  const bOpened = b.last_opened_at ? new Date(b.last_opened_at).getTime() : 0;
  if (bOpened !== aOpened) return bOpened - aOpened;

  const aMod = a.last_modified_at ? new Date(a.last_modified_at).getTime() : 0;
  const bMod = b.last_modified_at ? new Date(b.last_modified_at).getTime() : 0;
  if (bMod !== aMod) return bMod - aMod;

  return a.name.localeCompare(b.name);
}

function childrenOf(
  parentId: string,
  projects: ProjectListItem[],
): ProjectListItem[] {
  return projects
    .filter((p) => p.parent_id === parentId)
    .sort(sortByRecency);
}

function isRootInSet(project: ProjectListItem, visibleIds: Set<string>): boolean {
  if (!project.parent_id) return true;
  return !visibleIds.has(project.parent_id);
}

export function getChildProjects(
  parentId: string,
  projects: ProjectListItem[],
): ProjectListItem[] {
  return childrenOf(parentId, projects).sort(sortByRecency);
}

export function buildVisibleProjectRows(
  projects: ProjectListItem[],
  expandedIds: Set<string>,
  searchQuery: string,
  filterGroup: string,
  filterLanguage: string,
  filterFolder: string,
): VisibleProjectRow[] {
  const filtered = projects.filter((p) =>
    passesFilters(p, searchQuery, filterGroup, filterLanguage, filterFolder),
  );
  const visibleIds = new Set(filtered.map((p) => p.id));
  const searching = searchQuery.trim().length > 0;

  if (searching) {
    return filtered.sort(sortByRecency).map((project) => ({
      project,
      depth: project.depth,
    }));
  }

  const roots = filtered
    .filter((p) => isRootInSet(p, visibleIds))
    .sort(sortByRecency);

  const rows: VisibleProjectRow[] = [];

  const walk = (project: ProjectListItem, depth: number) => {
    rows.push({ project, depth });
    if (expandedIds.has(project.id)) {
      for (const child of childrenOf(project.id, filtered)) {
        walk(child, depth + 1);
      }
    }
  };

  for (const root of roots) {
    walk(root, 0);
  }

  return rows;
}
