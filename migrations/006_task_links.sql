-- External links of tasks: GitHub issues and the agent / tool that created them.
CREATE TABLE IF NOT EXISTS task_links (
    task_id TEXT PRIMARY KEY,
    github_repo TEXT,
    github_issue_number INTEGER,
    github_issue_url TEXT,
    github_issue_state TEXT,
    source TEXT,
    external_ref TEXT,
    synced_at TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_task_links_issue ON task_links(github_repo, github_issue_number);

CREATE INDEX IF NOT EXISTS idx_task_links_source ON task_links(source, external_ref);
