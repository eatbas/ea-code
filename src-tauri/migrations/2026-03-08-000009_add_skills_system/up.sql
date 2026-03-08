-- Phase 3 skills system: skill catalogue plus selector settings.
CREATE TABLE skills (
    id              TEXT NOT NULL PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    instructions    TEXT NOT NULL DEFAULT '',
    tags            TEXT NOT NULL DEFAULT '',
    is_active       BOOLEAN NOT NULL DEFAULT 1,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_skills_name ON skills(name COLLATE NOCASE);
CREATE INDEX idx_skills_active ON skills(is_active, updated_at DESC);

ALTER TABLE settings ADD COLUMN skill_selector_agent TEXT;
ALTER TABLE settings ADD COLUMN skill_selector_model TEXT;
ALTER TABLE settings ADD COLUMN skill_selection_mode TEXT NOT NULL DEFAULT 'disable';
