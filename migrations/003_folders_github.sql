-- Virtual folders and GitHub integration

CREATE TABLE IF NOT EXISTS folders (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    parent_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES folders(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS project_folder_assignments (
    project_id TEXT PRIMARY KEY,
    folder_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE
);

ALTER TABLE projects ADD COLUMN github_repo_id INTEGER;
ALTER TABLE projects ADD COLUMN remote_url TEXT;

CREATE TABLE IF NOT EXISTS github_repos_cache (
    id INTEGER PRIMARY KEY,
    full_name TEXT NOT NULL UNIQUE,
    clone_url TEXT NOT NULL,
    ssh_url TEXT,
    html_url TEXT,
    description TEXT,
    private INTEGER NOT NULL DEFAULT 0,
    default_branch TEXT,
    updated_at TEXT NOT NULL,
    synced_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_folders_parent_id ON folders(parent_id);
CREATE INDEX IF NOT EXISTS idx_github_repos_full_name ON github_repos_cache(full_name);
