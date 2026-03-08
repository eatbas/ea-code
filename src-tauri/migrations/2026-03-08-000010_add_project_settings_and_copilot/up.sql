-- Phase B: project-scoped settings overrides + Copilot CLI support.

CREATE TABLE project_settings (
    id              INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    setting_key     TEXT NOT NULL,
    setting_value   TEXT NOT NULL,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(project_id, setting_key)
);

CREATE INDEX idx_project_settings_project ON project_settings(project_id, updated_at DESC);

ALTER TABLE settings ADD COLUMN copilot_path TEXT NOT NULL DEFAULT 'gh';
ALTER TABLE settings ADD COLUMN copilot_model TEXT NOT NULL DEFAULT 'default';
