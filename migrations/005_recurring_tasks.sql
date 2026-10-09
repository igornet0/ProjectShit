-- Recurring tasks: a repeat rule on each instance plus a shared series id.
ALTER TABLE tasks ADD COLUMN recurrence_frequency TEXT;

ALTER TABLE tasks ADD COLUMN recurrence_interval INTEGER;

ALTER TABLE tasks ADD COLUMN recurrence_start TEXT;

ALTER TABLE tasks ADD COLUMN recurrence_until TEXT;

ALTER TABLE tasks ADD COLUMN series_id TEXT;

CREATE INDEX IF NOT EXISTS idx_tasks_series_id ON tasks(series_id);
