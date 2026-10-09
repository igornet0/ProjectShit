-- Personal tasks: project_id becomes optional.
-- SQLite cannot drop NOT NULL in place, so the table is rebuilt. Foreign keys
-- are switched off for the rebuild so dropping the old table does not null out
-- calendar_events.task_id.
PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE tasks_new (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'todo',
    priority TEXT NOT NULL DEFAULT 'medium',
    due_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

INSERT INTO tasks_new (id, project_id, title, description, status, priority, due_at, created_at, updated_at)
SELECT id, project_id, title, description, status, priority, due_at, created_at, updated_at FROM tasks;

DROP TABLE tasks;

ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);

CREATE INDEX IF NOT EXISTS idx_tasks_due_at ON tasks(due_at);

COMMIT;

PRAGMA foreign_keys = ON;
