ALTER TABLE settings ADD COLUMN kimi_path TEXT NOT NULL DEFAULT 'kimi';
ALTER TABLE settings ADD COLUMN opencode_path TEXT NOT NULL DEFAULT 'opencode';
ALTER TABLE settings ADD COLUMN kimi_model TEXT NOT NULL DEFAULT 'kimi-k2.5';
ALTER TABLE settings ADD COLUMN opencode_model TEXT NOT NULL DEFAULT 'opencode/glm-5';
