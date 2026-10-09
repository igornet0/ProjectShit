import { invoke } from "@tauri-apps/api/core";
import type {
  Activity,
  BrddBundle,
  BrddRefreshAll,
  BrddReport,
  BrddSettings,
  IssueSyncReport,
  CalendarEvent,
  EditorConfig,
  EditorDefinition,
  Folder,
  GitCommitInfo,
  GitHubConfig,
  GitHubOAuthStart,
  GitHubRepo,
  GitStatus,
  HubProjectEntry,
  Project,
  ProjectDetail,
  ProjectListItem,
  ProjectRoot,
  RecurrenceFrequency,
  Task,
} from "@/types";

export const api = {
  projects: {
    list: () => invoke<Project[]>("projects_list"),
    listSummaries: (includeArchived?: boolean) =>
      invoke<ProjectListItem[]>("projects_list_summaries", { includeArchived }).then(
        (items) => items.map(normalizeProjectListItem),
      ),
    listGroups: () => invoke<string[]>("projects_list_groups"),
    get: (id: string) => invoke<Project>("projects_get", { id }),
    search: (query: string) => invoke<Project[]>("projects_search", { query }),
    scan: (path: string) =>
      invoke<{ projects: Project[] }>("projects_scan", { path }),
    refresh: (id: string) => invoke<Project>("projects_refresh", { id }),
    getDetail: (id: string) =>
      invoke<ProjectDetail>("projects_get_detail", { id }),
    delete: (id: string) => invoke<void>("projects_delete", { id }),
    openFolder: (id: string) => invoke<void>("projects_open_folder", { id }),
    markOpened: (id: string) => invoke<void>("projects_mark_opened", { id }),
    update: (input: {
      id: string;
      group_name?: string | null;
      status?: string;
    }) => invoke<Project>("projects_update", { input }),
    openInEditor: (id: string, editorId?: string) =>
      invoke<void>("projects_open_in_editor", { id, editorId }),
    roots: {
      list: () => invoke<ProjectRoot[]>("project_roots_list"),
      add: (path: string) =>
        invoke<{ root: ProjectRoot; projects: Project[] }>(
          "project_roots_add",
          { path },
        ),
    },
  },

  tasks: {
    list: () => invoke<Task[]>("tasks_list"),
    listByProject: (projectId: string) =>
      invoke<Task[]>("tasks_list_by_project", { projectId }),
    create: (input: {
      project_id?: string | null;
      title: string;
      description?: string;
      status?: Task["status"];
      priority?: Task["priority"];
      due_at?: string;
      recurrence?: { frequency: RecurrenceFrequency; interval?: number; until?: string };
    }) => invoke<Task>("tasks_create", { input }),
    update: (input: {
      id: string;
      project_id?: string;
      title?: string;
      description?: string;
      status?: Task["status"];
      priority?: Task["priority"];
      due_at?: string;
      clear_due_at?: boolean;
      clear_recurrence?: boolean;
    }) => invoke<Task>("tasks_update", { input }),
    delete: (id: string) => invoke<void>("tasks_delete", { id }),
    createGithubIssue: (id: string, labels?: string[]) =>
      invoke<unknown>("tasks_create_github_issue", { id, labels }),
  },

  brdd: {
    get: (projectId: string) => invoke<BrddBundle>("brdd_get", { projectId }),
    refresh: (projectId: string, forceSnapshot = false) =>
      invoke<BrddReport>("brdd_refresh", { projectId, forceSnapshot }),
    refreshAll: () => invoke<BrddRefreshAll>("brdd_refresh_all"),
    saveNotes: (projectId: string, markdown: string) =>
      invoke<BrddBundle>("brdd_save_notes", { projectId, markdown }),
    getSettings: () => invoke<BrddSettings>("brdd_get_settings"),
    saveSettings: (settings: BrddSettings) =>
      invoke<BrddSettings>("brdd_save_settings", { settings }),
  },

  calendar: {
    list: () => invoke<CalendarEvent[]>("calendar_list"),
    create: (input: {
      project_id?: string;
      task_id?: string;
      title: string;
      start_at: string;
      end_at?: string;
    }) => invoke<CalendarEvent>("calendar_create", { input }),
  },

  git: {
    status: (projectId: string) =>
      invoke<{ status: GitStatus; recent_commits: GitCommitInfo[] }>(
        "git_status",
        { projectId },
      ),
  },

  commands: {
    run: (projectId: string, command: string) =>
      invoke<{ exit_code: number; stdout: string; stderr: string }>(
        "commands_run",
        { projectId, command },
      ),
  },

  activity: {
    list: (limit?: number) => invoke<Activity[]>("activity_list", { limit }),
    listByProject: (projectId: string, limit?: number) =>
      invoke<Activity[]>("activity_list_by_project", { projectId, limit }),
  },

  editors: {
    getConfig: () => invoke<EditorConfig>("editors_get_config"),
    saveConfig: (input: {
      default_editor_id?: string | null;
      editors: EditorDefinition[];
    }) => invoke<EditorConfig>("editors_save_config", { input }),
    addCustom: (input: {
      id: string;
      name: string;
      command: string;
      args: string[];
    }) => invoke<EditorConfig>("editors_add_custom", { input }),
  },

  folders: {
    list: () =>
      invoke<Folder[]>("folders_list").then((items) => items.map(normalizeFolder)),
    create: (input: { name: string; parent_id?: string | null }) =>
      invoke<Folder>("folders_create", { input }).then(normalizeFolder),
    rename: (input: { id: string; name: string }) =>
      invoke<void>("folders_rename", { input }),
    delete: (id: string) => invoke<void>("folders_delete", { id }),
    assignProject: (input: { project_id: string; folder_id?: string | null }) =>
      invoke<void>("folders_assign_project", { input }),
    reorder: (folder_ids: string[]) =>
      invoke<void>("folders_reorder", { input: { folder_ids } }),
  },

  github: {
    getConfig: () => invoke<GitHubConfig>("github_get_config"),
    oauthStart: () => invoke<GitHubOAuthStart>("github_oauth_start"),
    oauthComplete: (input?: { default_clone_dir?: string | null }) =>
      invoke<GitHubConfig>("github_oauth_complete", { input: input ?? {} }),
    connect: (input: { token: string; default_clone_dir?: string | null }) =>
      invoke<GitHubConfig>("github_connect", { input }),
    disconnect: () => invoke<void>("github_disconnect"),
    saveConfig: (config: {
      username?: string | null;
      default_clone_dir?: string | null;
      connected_at?: string | null;
    }) => invoke<GitHubConfig>("github_save_config", { config }),
    syncRepos: () => invoke<GitHubRepo[]>("github_sync_repos"),
    cloneRepo: (input: { full_name: string; target_dir?: string | null }) =>
      invoke<Project>("github_clone_repo", { input }),
    syncIssues: () => invoke<IssueSyncReport>("github_issues_sync"),
    listHub: (includeArchived?: boolean) =>
      invoke<HubProjectEntry[]>("projects_list_hub", { includeArchived }),
  },
};

function normalizeFolder(raw: Folder): Folder {
  return {
    ...raw,
    id: String(raw.id),
    parent_id: raw.parent_id ? String(raw.parent_id) : null,
  };
}

function normalizeProjectListItem(raw: ProjectListItem): ProjectListItem {
  return {
    ...raw,
    id: String(raw.id),
    parent_id: raw.parent_id ? String(raw.parent_id) : null,
    folder_id: raw.folder_id ? String(raw.folder_id) : null,
  };
}
