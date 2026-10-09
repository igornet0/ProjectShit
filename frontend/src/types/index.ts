export type Language =
  | "rust"
  | "python"
  | "javascript"
  | "typescript"
  | "go"
  | "java"
  | "cpp"
  | "unknown";

export type ProjectType =
  | "application"
  | "library"
  | "workspace"
  | "web"
  | "embedded"
  | "unknown";

export type ProjectStatus = "active" | "paused" | "archived";

export interface Project {
  id: string;
  name: string;
  root_path: string;
  language: Language;
  project_type: ProjectType;
  status: ProjectStatus;
  description: string | null;
  icon: string | null;
  group_name: string | null;
  github_repo_id: number | null;
  remote_url: string | null;
  last_opened_at: string | null;
  last_modified_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectListItem {
  id: string;
  name: string;
  root_path: string;
  language: Language;
  project_type: ProjectType;
  status: ProjectStatus;
  description: string | null;
  icon: string | null;
  group_name: string | null;
  github_repo_id: number | null;
  remote_url: string | null;
  folder_id: string | null;
  last_opened_at: string | null;
  last_modified_at: string | null;
  created_at: string;
  updated_at: string;
  has_git: boolean;
  git_dirty: boolean;
  parent_id: string | null;
  child_count: number;
  depth: number;
}

export interface Folder {
  id: string;
  name: string;
  parent_id: string | null;
  sort_order: number;
  created_at: string;
  project_count: number;
}

export interface GitHubRepo {
  id: number;
  full_name: string;
  clone_url: string;
  ssh_url: string | null;
  html_url: string;
  description: string | null;
  private: boolean;
  default_branch: string | null;
  updated_at: string;
}

export interface GitHubConfig {
  username: string | null;
  default_clone_dir: string | null;
  connected_at: string | null;
  connected: boolean;
}

export interface GitHubOAuthStart {
  user_code: string;
  verification_uri: string;
}

export interface HubProjectEntry {
  kind: "local" | "ghost";
  project: ProjectListItem | null;
  github_repo: GitHubRepo;
}

export interface EditorDefinition {
  id: string;
  name: string;
  command: string;
  args: string[];
  builtin: boolean;
}

export interface EditorConfig {
  default_editor_id: string | null;
  editors: EditorDefinition[];
}

export interface ProjectRoot {
  id: string;
  path: string;
  created_at: string;
}

export const TASK_STATUSES = ["todo", "in_progress", "done", "cancelled"] as const;
export const TASK_PRIORITIES = ["low", "medium", "high", "critical"] as const;

export const RECURRENCE_FREQUENCIES = ["daily", "weekly", "monthly", "quarterly", "yearly"] as const;
export type RecurrenceFrequency = (typeof RECURRENCE_FREQUENCIES)[number];

export interface Recurrence {
  frequency: RecurrenceFrequency;
  interval: number;
  /** First occurrence's due date; later occurrences are computed from it. */
  start: string;
  until: string | null;
}

export interface Task {
  id: string;
  /** `null` for personal tasks. */
  project_id: string | null;
  title: string;
  description: string | null;
  status: (typeof TASK_STATUSES)[number];
  priority: (typeof TASK_PRIORITIES)[number];
  due_at: string | null;
  recurrence: Recurrence | null;
  series_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface CalendarEvent {
  id: string;
  project_id: string | null;
  task_id: string | null;
  title: string;
  start_at: string;
  end_at: string | null;
  created_at: string;
}

export interface GitStatus {
  branch: string;
  modified: number;
  staged: number;
  untracked: number;
  ahead: number;
  behind: number;
}

export interface GitCommitInfo {
  hash: string;
  message: string;
  author: string;
  date: string;
}

export interface ProjectDetail {
  project: Project;
  packages: string[];
  commands: { name: string; command: string }[];
}

export interface Activity {
  id: string;
  project_id: string | null;
  activity_type: string;
  message: string | null;
  metadata: unknown;
  created_at: string;
}

export const LANGUAGE_ICONS: Record<string, string> = {
  rust: "🦀",
  python: "🐍",
  javascript: "📦",
  typescript: "📘",
  go: "🐹",
  java: "☕",
  cpp: "⚙️",
  unknown: "📁",
};

export const STATUS_COLORS: Record<ProjectStatus, string> = {
  active: "#22c55e",
  paused: "#eab308",
  archived: "#6b7280",
};

// ─── .brdd (per-project analysis folder) ─────────────────────────────────────

export interface BrddVersionSnapshot {
  n: number;
  at: string;
  version: string | null;
  branch: string | null;
  commit: string | null;
  commit_short: string | null;
  tags: string[];
  dirty: boolean;
  reason: "initial" | "version" | "commits" | "manual" | string;
  files: number;
  loc: number;
  commits: GitCommitInfo[];
  diff?: { files_changed: number; insertions: number; deletions: number } | null;
}

export interface BrddAnalysis {
  format: number;
  analyzed_at: string;
  description: string | null;
  version: string | null;
  version_source: string | null;
  manifests: string[];
  stack: string[];
  dependencies: { name: string; version?: string | null; kind: string; source: string }[];
  structure: {
    files: number;
    source_files: number;
    loc: number;
    languages: { language: string; files: number; loc: number }[];
    has_tests: boolean;
    has_docker: boolean;
    ci: string[];
    truncated: boolean;
  };
  tasks: { total: number; todo: number; in_progress: number; done: number; cancelled: number };
}

export interface BrddBundle {
  exists: boolean;
  dir: string;
  analysis: BrddAnalysis | null;
  versions: BrddVersionSnapshot[];
  summary_md: string | null;
  changelog_md: string | null;
  notes_md: string | null;
}

export interface BrddReport {
  dir: string;
  analysis: BrddAnalysis;
  snapshot: BrddVersionSnapshot | null;
  snapshots: number;
  files_written: string[];
}

export interface BrddSettings {
  auto_on_scan: boolean;
  exclude_from_git: boolean;
}

export interface BrddRefreshAll {
  refreshed: number;
  failed: number;
  items: { project_id: string; name: string; ok: boolean; error?: string | null; snapshot?: number | null }[];
}

export interface IssueSyncReport {
  checked: number;
  tasks_completed: number;
  tasks_reopened: number;
  issues_closed: number;
  errors: string[];
}
