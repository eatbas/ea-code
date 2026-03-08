-- Phase 2 settings: plan gate, retry, token optimisation.
ALTER TABLE settings ADD COLUMN require_plan_approval BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN plan_auto_approve_timeout_sec INTEGER NOT NULL DEFAULT 45;
ALTER TABLE settings ADD COLUMN max_plan_revisions INTEGER NOT NULL DEFAULT 3;
ALTER TABLE settings ADD COLUMN token_optimized_prompts BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN agent_retry_count INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN agent_timeout_ms INTEGER NOT NULL DEFAULT 0;

-- Track plan gate decisions per iteration.
ALTER TABLE iterations ADD COLUMN plan_approval TEXT;
ALTER TABLE iterations ADD COLUMN plan_revision_count INTEGER NOT NULL DEFAULT 0;
