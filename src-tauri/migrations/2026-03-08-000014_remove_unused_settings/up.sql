-- Remove unused pipeline settings columns.
ALTER TABLE settings DROP COLUMN skill_selection_mode;
ALTER TABLE settings DROP COLUMN mode;
ALTER TABLE settings DROP COLUMN update_cli_on_run;
ALTER TABLE settings DROP COLUMN fail_on_cli_update_error;
ALTER TABLE settings DROP COLUMN cli_update_timeout_ms;
