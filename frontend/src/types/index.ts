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

export interface Task {
  id: string;
  project_id: string;
  title: string;
  description: string | null;
  status: "todo" | "in_progress" | "done" | "cancelled";
  priority: "low" | "medium" | "high" | "critical";
  due_at: string | null;
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
