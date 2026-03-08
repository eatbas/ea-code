-- Add typed pipeline context fields and run-level executive summary fields.

ALTER TABLE iterations ADD COLUMN enhanced_prompt TEXT;
ALTER TABLE iterations ADD COLUMN planner_plan TEXT;
ALTER TABLE iterations ADD COLUMN audit_verdict TEXT;
ALTER TABLE iterations ADD COLUMN audit_reasoning TEXT;
ALTER TABLE iterations ADD COLUMN audited_plan TEXT;
ALTER TABLE iterations ADD COLUMN review_output TEXT;
ALTER TABLE iterations ADD COLUMN review_user_guidance TEXT;
ALTER TABLE iterations ADD COLUMN fix_output TEXT;
ALTER TABLE iterations ADD COLUMN judge_output TEXT;
ALTER TABLE iterations ADD COLUMN generate_question TEXT;
ALTER TABLE iterations ADD COLUMN generate_answer TEXT;
ALTER TABLE iterations ADD COLUMN fix_question TEXT;
ALTER TABLE iterations ADD COLUMN fix_answer TEXT;

ALTER TABLE runs ADD COLUMN executive_summary TEXT;
ALTER TABLE runs ADD COLUMN executive_summary_status TEXT;
ALTER TABLE runs ADD COLUMN executive_summary_error TEXT;
ALTER TABLE runs ADD COLUMN executive_summary_agent TEXT;
ALTER TABLE runs ADD COLUMN executive_summary_model TEXT;
ALTER TABLE runs ADD COLUMN executive_summary_generated_at TIMESTAMP;

ALTER TABLE settings ADD COLUMN executive_summary_agent TEXT NOT NULL DEFAULT 'codex';
ALTER TABLE settings ADD COLUMN executive_summary_model TEXT NOT NULL DEFAULT 'codex-5.3';
