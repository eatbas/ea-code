-- Re-add removed pipeline settings columns with original defaults.
ALTER TABLE settings ADD COLUMN skill_selection_mode TEXT NOT NULL DEFAULT 'disable';
ALTER TABLE settings ADD COLUMN mode TEXT NOT NULL DEFAULT 'workspace-write';
ALTER TABLE settings ADD COLUMN update_cli_on_run BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN fail_on_cli_update_error BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN cli_update_timeout_ms INTEGER NOT NULL DEFAULT 600000;
