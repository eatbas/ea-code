-- Per-stage model selection for the Agents view.
ALTER TABLE settings ADD COLUMN prompt_enhancer_model TEXT NOT NULL DEFAULT 'sonnet';
ALTER TABLE settings ADD COLUMN planner_model TEXT;
ALTER TABLE settings ADD COLUMN plan_auditor_model TEXT;
ALTER TABLE settings ADD COLUMN generator_model TEXT NOT NULL DEFAULT 'sonnet';
ALTER TABLE settings ADD COLUMN reviewer_model TEXT NOT NULL DEFAULT 'codex-5.3';
ALTER TABLE settings ADD COLUMN fixer_model TEXT NOT NULL DEFAULT 'sonnet';
ALTER TABLE settings ADD COLUMN final_judge_model TEXT NOT NULL DEFAULT 'codex-5.3';
