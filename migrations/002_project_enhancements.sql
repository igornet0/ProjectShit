-- Project enhancements: groups, recency, settings

ALTER TABLE projects ADD COLUMN group_name TEXT;
ALTER TABLE projects ADD COLUMN last_opened_at TEXT;
ALTER TABLE projects ADD COLUMN last_modified_at TEXT;

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_projects_last_opened_at ON projects(last_opened_at);
CREATE INDEX IF NOT EXISTS idx_projects_group_name ON projects(group_name);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
