ALTER TABLE settings ADD COLUMN claude_model TEXT NOT NULL DEFAULT 'sonnet';
ALTER TABLE settings ADD COLUMN codex_model TEXT NOT NULL DEFAULT 'o4-mini';
ALTER TABLE settings ADD COLUMN gemini_model TEXT NOT NULL DEFAULT 'gemini-2.5-pro';
