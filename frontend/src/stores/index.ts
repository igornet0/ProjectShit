import { create } from "zustand";
import { api } from "@/api";
import { buildVisibleProjectRows, getChildProjects } from "@/utils/projectTree";
import type {
  CalendarEvent,
  EditorConfig,
  Folder,
  GitHubConfig,
  HubProjectEntry,
  Project,
  ProjectListItem,
  ProjectRoot,
  Task,
} from "@/types";

interface ProjectStore {
  projects: ProjectListItem[];
  groups: string[];
  roots: ProjectRoot[];
  loading: boolean;
  error: string | null;
  searchQuery: string;
  filterGroup: string;
  filterLanguage: string;
  filterFolder: string;
  showArchived: boolean;
  showGithub: boolean;
  expandedProjectIds: string[];

  /** Reuses fresh data and in-flight requests unless `force` is set. */
  fetchProjects: (force?: boolean) => Promise<void>;
  fetchGroups: () => Promise<void>;
  fetchRoots: () => Promise<void>;
  addRoot: (path: string) => Promise<void>;
  removeProject: (id: string) => Promise<void>;
  openProjectFolder: (id: string) => Promise<void>;
  openInEditor: (id: string, editorId?: string) => Promise<void>;
  markOpened: (id: string) => Promise<void>;
  updateProject: (input: {
    id: string;
    group_name?: string | null;
    status?: string;
  }) => Promise<Project>;
  setSearchQuery: (query: string) => void;
  setFilterGroup: (group: string) => void;
  setFilterLanguage: (language: string) => void;
  setFilterFolder: (folder: string) => void;
  setShowArchived: (show: boolean) => void;
  setShowGithub: (show: boolean) => void;
  assignToFolder: (projectId: string, folderId: string | null) => Promise<void>;
  toggleProjectExpanded: (id: string) => void;
  expandProject: (id: string) => void;
  filteredProjects: () => ProjectListItem[];
  visibleProjectRows: () => ReturnType<typeof buildVisibleProjectRows>;
  getChildProjects: (parentId: string) => ProjectListItem[];
}

const PROJECTS_STALE_MS = 10_000;
let projectsRequest: Promise<void> | null = null;
let projectsFetchedAt = 0;
let projectsFetchedArchived = false;

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  groups: [],
  roots: [],
  loading: false,
  error: null,
  searchQuery: "",
  filterGroup: "all",
  filterLanguage: "all",
  filterFolder: "all",
  showArchived: false,
  showGithub: false,
  expandedProjectIds: [],

  fetchProjects: async (force = false) => {
    const { showArchived, projects } = get();
    const fresh =
      projectsFetchedArchived === showArchived &&
      Date.now() - projectsFetchedAt < PROJECTS_STALE_MS;
    if (!force && fresh) return;
    if (!force && projectsRequest) return projectsRequest;

    // Only show the spinner on first load; later refreshes are silent.
    set({ loading: projects.length === 0, error: null });
    const request = (async () => {
      try {
        const items = await api.projects.listSummaries(showArchived);
        projectsFetchedAt = Date.now();
        projectsFetchedArchived = showArchived;
        set({ projects: items, loading: false });
      } catch (e) {
        set({ error: String(e), loading: false });
      }
    })();
    void request.finally(() => {
      if (projectsRequest === request) projectsRequest = null;
    });
    projectsRequest = request;
    return request;
  },

  fetchGroups: async () => {
    try {
      const groups = await api.projects.listGroups();
      set({ groups });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchRoots: async () => {
    try {
      const roots = await api.projects.roots.list();
      set({ roots });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addRoot: async (path: string) => {
    set({ loading: true, error: null });
    try {
      await api.projects.roots.add(path);
      await Promise.all([get().fetchProjects(true), get().fetchGroups()]);
      set({ loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  removeProject: async (id: string) => {
    set({ error: null });
    try {
      await api.projects.delete(id);
      set((state) => ({
        projects: state.projects.filter((p) => p.id !== id),
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  openProjectFolder: async (id: string) => {
    try {
      await api.projects.openFolder(id);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  openInEditor: async (id: string, editorId?: string) => {
    try {
      await api.projects.openInEditor(id, editorId);
      await get().markOpened(id);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  markOpened: async (id: string) => {
    try {
      await api.projects.markOpened(id);
      set((state) => ({
        projects: sortProjects(
          state.projects.map((p) =>
            p.id === id
              ? { ...p, last_opened_at: new Date().toISOString() }
              : p,
          ),
        ),
      }));
    } catch {
      /* non-critical */
    }
  },

  updateProject: async (input) => {
    const updated = await api.projects.update(input);
    set((state) => ({
      projects: sortProjects(
        state.projects.map((p) =>
          p.id === updated.id
            ? { ...p, ...updated, has_git: p.has_git, git_dirty: p.git_dirty }
            : p,
        ),
      ),
    }));
    await get().fetchGroups();
    return updated;
  },

  setSearchQuery: (query: string) => {
    set({ searchQuery: query });
  },

  setFilterGroup: (group: string) => set({ filterGroup: group }),

  setFilterLanguage: (language: string) => set({ filterLanguage: language }),

  setFilterFolder: (folder: string) => set({ filterFolder: folder }),

  setShowGithub: (show: boolean) => set({ showGithub: show }),

  assignToFolder: async (projectId, folderId) => {
    const previousProjects = get().projects;
    const previousFolders = useFolderStore.getState().folders;
    const previousFolderId =
      previousProjects.find((p) => p.id === projectId)?.folder_id ?? null;

    set({
      error: null,
      projects: previousProjects.map((p) =>
        p.id === projectId ? { ...p, folder_id: folderId } : p,
      ),
    });

    useFolderStore.setState({
      folders: previousFolders.map((f) => {
        let count = f.project_count;
        if (
          previousFolderId &&
          f.id === previousFolderId &&
          previousFolderId !== folderId
        ) {
          count = Math.max(0, count - 1);
        }
        if (folderId && f.id === folderId && previousFolderId !== folderId) {
          count += 1;
        }
        return count === f.project_count ? f : { ...f, project_count: count };
      }),
    });

    try {
      await api.folders.assignProject({
        project_id: projectId,
        folder_id: folderId,
      });
      await useFolderStore.getState().fetchFolders(true);
    } catch (e) {
      set({ error: String(e), projects: previousProjects });
      useFolderStore.setState({ folders: previousFolders });
      throw e;
    }
  },

  setShowArchived: (show: boolean) => {
    set({ showArchived: show });
    get().fetchProjects(true);
  },

  toggleProjectExpanded: (id: string) => {
    set((state) => {
      const expanded = new Set(state.expandedProjectIds);
      if (expanded.has(id)) {
        expanded.delete(id);
      } else {
        expanded.add(id);
      }
      return { expandedProjectIds: [...expanded] };
    });
  },

  expandProject: (id: string) => {
    set((state) => {
      if (state.expandedProjectIds.includes(id)) return state;
      return { expandedProjectIds: [...state.expandedProjectIds, id] };
    });
  },

  filteredProjects: () => {
    const { projects, searchQuery, filterGroup, filterLanguage } = get();
    const q = searchQuery.trim().toLowerCase();

    return projects.filter((p) => {
      if (q && !p.name.toLowerCase().includes(q) && !p.root_path.toLowerCase().includes(q)) {
        return false;
      }
      if (filterGroup !== "all") {
        if (filterGroup === "ungrouped") {
          if (p.group_name) return false;
        } else if (p.group_name !== filterGroup) {
          return false;
        }
      }
      if (filterLanguage !== "all" && p.language !== filterLanguage) {
        return false;
      }
      return true;
    });
  },

  visibleProjectRows: () => {
    const {
      projects,
      expandedProjectIds,
      searchQuery,
      filterGroup,
      filterLanguage,
      filterFolder,
    } = get();
    return buildVisibleProjectRows(
      projects,
      new Set(expandedProjectIds),
      searchQuery,
      filterGroup,
      filterLanguage,
      filterFolder,
    );
  },

  getChildProjects: (parentId: string) => {
    return getChildProjects(parentId, get().projects);
  },
}));

function sortProjects(projects: ProjectListItem[]): ProjectListItem[] {
  return [...projects].sort((a, b) => {
    const aOpened = a.last_opened_at ? new Date(a.last_opened_at).getTime() : 0;
    const bOpened = b.last_opened_at ? new Date(b.last_opened_at).getTime() : 0;
    if (bOpened !== aOpened) return bOpened - aOpened;

    const aMod = a.last_modified_at ? new Date(a.last_modified_at).getTime() : 0;
    const bMod = b.last_modified_at ? new Date(b.last_modified_at).getTime() : 0;
    if (bMod !== aMod) return bMod - aMod;

    return a.name.localeCompare(b.name);
  });
}

interface EditorStore {
  config: EditorConfig | null;
  loading: boolean;
  fetchConfig: () => Promise<void>;
  saveConfig: (config: EditorConfig) => Promise<void>;
  addCustomEditor: (input: {
    id: string;
    name: string;
    command: string;
    args: string[];
  }) => Promise<void>;
}

export const useEditorStore = create<EditorStore>((set) => ({
  config: null,
  loading: false,

  fetchConfig: async () => {
    set({ loading: true });
    try {
      const config = await api.editors.getConfig();
      set({ config, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  saveConfig: async (config) => {
    const saved = await api.editors.saveConfig({
      default_editor_id: config.default_editor_id,
      editors: config.editors,
    });
    set({ config: saved });
  },

  addCustomEditor: async (input) => {
    const config = await api.editors.addCustom(input);
    set({ config });
  },
}));

interface TaskStore {
  tasks: Task[];
  loading: boolean;
  error: string | null;

  fetchTasks: () => Promise<void>;
  createTask: (input: Parameters<typeof api.tasks.create>[0]) => Promise<void>;
  toggleTask: (task: Task) => Promise<void>;
  updateTask: (input: Parameters<typeof api.tasks.update>[0]) => Promise<Task>;
  deleteTask: (id: string) => Promise<void>;
}

export const useTaskStore = create<TaskStore>((set, get) => ({
  tasks: [],
  loading: false,
  error: null,

  fetchTasks: async () => {
    set({ loading: true, error: null });
    try {
      const tasks = await api.tasks.list();
      set({ tasks, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createTask: async (input) => {
    try {
      const task = await api.tasks.create(input);
      set((state) => ({ tasks: [task, ...state.tasks] }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  toggleTask: async (task) => {
    try {
      await get().updateTask({ id: task.id, status: task.status === "done" ? "todo" : "done" });
    } catch {
      /* error stored */
    }
  },

  updateTask: async (input) => {
    const before = get().tasks.find((t) => t.id === input.id);
    try {
      const updated = await api.tasks.update(input);
      if (before?.recurrence && updated.status === "done" && before.status !== "done") {
        // Completing a recurring task creates its next occurrence on the backend.
        set({ tasks: await api.tasks.list() });
      } else {
        set((state) => ({
          tasks: state.tasks.some((t) => t.id === updated.id)
            ? state.tasks.map((t) => (t.id === updated.id ? updated : t))
            : [updated, ...state.tasks],
        }));
      }
      return updated;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteTask: async (id) => {
    try {
      await api.tasks.delete(id);
      set((state) => ({ tasks: state.tasks.filter((t) => t.id !== id) }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

}));

interface CalendarStore {
  events: CalendarEvent[];
  loading: boolean;

  fetchEvents: () => Promise<void>;
}

export const useCalendarStore = create<CalendarStore>((set) => ({
  events: [],
  loading: false,

  fetchEvents: async () => {
    set({ loading: true });
    try {
      const events = await api.calendar.list();
      set({ events, loading: false });
    } catch {
      set({ loading: false });
    }
  },
}));

interface UiStore {
  commandPaletteOpen: boolean;
  setCommandPaletteOpen: (open: boolean) => void;
  /** Task shown in the global task dialog, if any. */
  openTaskId: string | null;
  openTask: (id: string | null) => void;
}

export const useUiStore = create<UiStore>((set) => ({
  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
  openTaskId: null,
  openTask: (id) => set({ openTaskId: id }),
}));

interface FolderStore {
  folders: Folder[];
  loading: boolean;
  error: string | null;
  fetchFolders: (silent?: boolean) => Promise<void>;
  createFolder: (name: string) => Promise<void>;
  deleteFolder: (id: string) => Promise<void>;
  reorderFolders: (orderedIds: string[]) => Promise<void>;
  clearError: () => void;
}

export const useFolderStore = create<FolderStore>((set, get) => ({
  folders: [],
  loading: false,
  error: null,

  clearError: () => set({ error: null }),

  fetchFolders: async (silent = false) => {
    if (!silent) {
      set({ loading: true, error: null });
    }
    try {
      const folders = await api.folders.list();
      set({ folders, loading: false });
    } catch (e) {
      console.error("Failed to load folders", e);
      set({ loading: false, error: String(e) });
    }
  },

  createFolder: async (name: string) => {
    set({ error: null });
    try {
      await api.folders.create({ name });
      await get().fetchFolders(true);
    } catch (e) {
      console.error("Failed to create folder", e);
      set({ error: String(e) });
      throw e;
    }
  },

  deleteFolder: async (id: string) => {
    set({ error: null });
    const previousFolders = get().folders;
    set({ folders: previousFolders.filter((f) => f.id !== id) });

    const projectStore = useProjectStore.getState();
    const previousProjects = projectStore.projects;
    useProjectStore.setState({
      projects: previousProjects.map((p) =>
        p.folder_id === id ? { ...p, folder_id: null } : p,
      ),
    });

    try {
      await api.folders.delete(id);
      await get().fetchFolders(true);
    } catch (e) {
      console.error("Failed to delete folder", e);
      set({ folders: previousFolders, error: String(e) });
      useProjectStore.setState({ projects: previousProjects });
      throw e;
    }
  },

  reorderFolders: async (orderedIds: string[]) => {
    const previous = get().folders;
    const byId = new Map(previous.map((f) => [f.id, f]));
    const reordered = orderedIds
      .map((id) => byId.get(id))
      .filter((f): f is Folder => f !== undefined);

    set({
      error: null,
      folders: reordered.map((f, index) => ({ ...f, sort_order: index })),
    });

    try {
      await api.folders.reorder(orderedIds);
      await get().fetchFolders(true);
    } catch (e) {
      console.error("Failed to reorder folders", e);
      set({ folders: previous, error: String(e) });
      throw e;
    }
  },
}));

interface GitHubStore {
  config: GitHubConfig | null;
  hubEntries: HubProjectEntry[];
  loading: boolean;
  authMessage: string | null;
  oauthUserCode: string | null;
  fetchConfig: () => Promise<void>;
  oauthLogin: () => Promise<void>;
  disconnect: () => Promise<void>;
  syncRepos: () => Promise<void>;
  fetchHubEntries: () => Promise<void>;
  cloneRepo: (fullName: string) => Promise<void>;
}

export const useGitHubStore = create<GitHubStore>((set) => ({
  config: null,
  hubEntries: [],
  loading: false,
  authMessage: null,
  oauthUserCode: null,

  fetchConfig: async () => {
    set({ loading: true });
    try {
      const config = await api.github.getConfig();
      set({ config, loading: false });
    } catch {
      set({ loading: false });
    }
  },

  oauthLogin: async () => {
    set({ loading: true, authMessage: null, oauthUserCode: null });
    try {
      const start = await api.github.oauthStart();
      set({ oauthUserCode: start.user_code });
      const config = await api.github.oauthComplete();
      set({ config, loading: false, authMessage: null, oauthUserCode: null });
      await useGitHubStore.getState().fetchHubEntries();
    } catch (e) {
      set({ loading: false, authMessage: String(e), oauthUserCode: null });
      throw e;
    }
  },

  disconnect: async () => {
    await api.github.disconnect();
    set({ config: null, hubEntries: [], authMessage: null });
  },

  syncRepos: async () => {
    set({ loading: true });
    try {
      await api.github.syncRepos();
      await Promise.all([
        useGitHubStore.getState().fetchHubEntries(),
        useProjectStore.getState().fetchProjects(true),
      ]);
      set({ loading: false });
    } catch {
      set({ loading: false });
    }
  },

  fetchHubEntries: async () => {
    try {
      const hubEntries = await api.github.listHub();
      set({ hubEntries });
    } catch {
      set({ hubEntries: [] });
    }
  },

  cloneRepo: async (fullName) => {
    set({ loading: true });
    try {
      await api.github.cloneRepo({ full_name: fullName });
      await Promise.all([
        useProjectStore.getState().fetchProjects(true),
        useGitHubStore.getState().fetchHubEntries(),
      ]);
      set({ loading: false });
    } catch {
      set({ loading: false });
      throw new Error("Clone failed");
    }
  },
}));
