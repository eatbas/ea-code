-- Consolidated initial schema for EA Code.
-- Settings, projects, sessions, runs, iterations, stages, artefacts, questions,
-- skills, MCP servers, CLI-MCP bindings.

PRAGMA foreign_keys = ON;

-- Settings: single-row config table.
CREATE TABLE settings (
    id                              INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    claude_path                     TEXT    NOT NULL DEFAULT 'claude',
    codex_path                      TEXT    NOT NULL DEFAULT 'codex',
    gemini_path                     TEXT    NOT NULL DEFAULT 'gemini',
    kimi_path                       TEXT    NOT NULL DEFAULT 'kimi',
    opencode_path                   TEXT    NOT NULL DEFAULT 'opencode',
    prompt_enhancer_agent           TEXT    NOT NULL DEFAULT 'claude',
    skill_selector_agent            TEXT,
    planner_agent                   TEXT,
    plan_auditor_agent              TEXT,
    generator_agent                 TEXT    NOT NULL DEFAULT 'claude',
    reviewer_agent                  TEXT    NOT NULL DEFAULT 'codex',
    fixer_agent                     TEXT    NOT NULL DEFAULT 'claude',
    final_judge_agent               TEXT    NOT NULL DEFAULT 'codex',
    max_iterations                  INTEGER NOT NULL DEFAULT 3,
    require_git                     BOOLEAN NOT NULL DEFAULT 1,
    updated_at                      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    claude_model                    TEXT    NOT NULL DEFAULT 'sonnet',
    codex_model                     TEXT    NOT NULL DEFAULT 'gpt-5.3-codex',
    gemini_model                    TEXT    NOT NULL DEFAULT 'gemini-3-flash-preview',
    kimi_model                      TEXT    NOT NULL DEFAULT 'kimi-code',
    opencode_model                  TEXT    NOT NULL DEFAULT 'opencode/glm-5',
    prompt_enhancer_model           TEXT    NOT NULL DEFAULT 'sonnet',
    skill_selector_model            TEXT,
    planner_model                   TEXT,
    plan_auditor_model              TEXT,
    generator_model                 TEXT    NOT NULL DEFAULT 'sonnet',
    reviewer_model                  TEXT    NOT NULL DEFAULT 'gpt-5.3-codex',
    fixer_model                     TEXT    NOT NULL DEFAULT 'sonnet',
    final_judge_model               TEXT    NOT NULL DEFAULT 'gpt-5.3-codex',
    executive_summary_agent         TEXT    NOT NULL DEFAULT 'codex',
    executive_summary_model         TEXT    NOT NULL DEFAULT 'gpt-5.3-codex',
    require_plan_approval           BOOLEAN NOT NULL DEFAULT 0,
    plan_auto_approve_timeout_sec   INTEGER NOT NULL DEFAULT 45,
    max_plan_revisions              INTEGER NOT NULL DEFAULT 3,
    token_optimized_prompts         BOOLEAN NOT NULL DEFAULT 0,
    agent_retry_count               INTEGER NOT NULL DEFAULT 1,
    agent_timeout_ms                INTEGER NOT NULL DEFAULT 0,
    agent_max_turns                 INTEGER NOT NULL DEFAULT 25,
    retention_days                  INTEGER NOT NULL DEFAULT 90
);

INSERT INTO settings (id) VALUES (1);

-- Projects: workspace bookmarks with git metadata.
CREATE TABLE projects (
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    path        TEXT    NOT NULL UNIQUE,
    name        TEXT    NOT NULL,
    is_git_repo BOOLEAN NOT NULL DEFAULT 0,
    branch      TEXT,
    last_opened TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_projects_last_opened ON projects(last_opened DESC);

-- Sessions: conversation threads (a "chat" in the sidebar).
CREATE TABLE sessions (
    id          TEXT      NOT NULL PRIMARY KEY,
    project_id  INTEGER   NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title       TEXT      NOT NULL DEFAULT 'New Session',
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_sessions_project ON sessions(project_id, updated_at DESC);

-- Runs: individual pipeline executions within a session.
CREATE TABLE runs (
    id                              TEXT      NOT NULL PRIMARY KEY,
    session_id                      TEXT      NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    prompt                          TEXT      NOT NULL,
    status                          TEXT      NOT NULL DEFAULT 'running',
    max_iterations                  INTEGER   NOT NULL DEFAULT 3,
    final_verdict                   TEXT,
    error                           TEXT,
    executive_summary               TEXT,
    executive_summary_status        TEXT,
    executive_summary_error         TEXT,
    executive_summary_agent         TEXT,
    executive_summary_model         TEXT,
    executive_summary_generated_at  TIMESTAMP,
    started_at                      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at                    TIMESTAMP,
    current_stage                   TEXT,
    current_iteration               INTEGER   NOT NULL DEFAULT 0,
    current_stage_started_at        TEXT
);

CREATE INDEX idx_runs_session ON runs(session_id, started_at ASC);
CREATE INDEX idx_runs_status_completed ON runs(status, completed_at);

-- Iterations: each loop of the self-improving pipeline.
CREATE TABLE iterations (
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    run_id              TEXT    NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    number              INTEGER NOT NULL,
    verdict             TEXT,
    judge_reasoning     TEXT,
    plan_approval       TEXT,
    plan_revision_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(run_id, number)
);

-- Stages: individual pipeline stages within an iteration.
CREATE TABLE stages (
    id              INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    iteration_id    INTEGER   NOT NULL REFERENCES iterations(id) ON DELETE CASCADE,
    stage           TEXT      NOT NULL,
    status          TEXT      NOT NULL,
    output          TEXT      NOT NULL DEFAULT '',
    duration_ms     INTEGER   NOT NULL DEFAULT 0,
    error           TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_stages_iteration ON stages(iteration_id);

-- Artefacts: diffs, reviews, validation output, judge reasoning.
CREATE TABLE artifacts (
    id          INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    run_id      TEXT      NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    iteration   INTEGER   NOT NULL,
    kind        TEXT      NOT NULL,
    content     TEXT      NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_artifacts_run ON artifacts(run_id, iteration);

-- Questions: user Q&A during pipeline pauses.
CREATE TABLE questions (
    id              TEXT      NOT NULL PRIMARY KEY,
    run_id          TEXT      NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    stage           TEXT      NOT NULL,
    iteration       INTEGER   NOT NULL,
    question_text   TEXT      NOT NULL,
    agent_output    TEXT      NOT NULL DEFAULT '',
    optional        BOOLEAN   NOT NULL DEFAULT 0,
    answer          TEXT,
    skipped         BOOLEAN   NOT NULL DEFAULT 0,
    asked_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    answered_at     TIMESTAMP
);

CREATE INDEX idx_questions_run ON questions(run_id);

-- Skills: skill catalogue for the pipeline.
CREATE TABLE skills (
    id              TEXT      NOT NULL PRIMARY KEY,
    name            TEXT      NOT NULL,
    description     TEXT      NOT NULL DEFAULT '',
    instructions    TEXT      NOT NULL DEFAULT '',
    tags            TEXT      NOT NULL DEFAULT '',
    is_active       BOOLEAN   NOT NULL DEFAULT 1,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_skills_name   ON skills(name COLLATE NOCASE);
CREATE INDEX idx_skills_active ON skills(is_active, updated_at DESC);

-- MCP server catalogue.
CREATE TABLE mcp_servers (
    id          TEXT      NOT NULL PRIMARY KEY,
    name        TEXT      NOT NULL,
    description TEXT      NOT NULL DEFAULT '',
    command     TEXT      NOT NULL,
    args        TEXT      NOT NULL DEFAULT '[]',
    env         TEXT      NOT NULL DEFAULT '{}',
    is_enabled  BOOLEAN   NOT NULL DEFAULT 0,
    is_builtin  BOOLEAN   NOT NULL DEFAULT 0,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_mcp_servers_enabled ON mcp_servers(is_enabled, is_builtin);

-- CLI-to-MCP bindings.
CREATE TABLE cli_mcp_bindings (
    id              INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    cli_name        TEXT      NOT NULL,
    mcp_server_id   TEXT      NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(cli_name, mcp_server_id)
);

CREATE INDEX idx_cli_mcp_bindings_cli ON cli_mcp_bindings(cli_name, mcp_server_id);

-- Built-in MCP catalogue.
INSERT INTO mcp_servers (id, name, description, command, args, env, is_enabled, is_builtin)
VALUES
  ('context7', 'Context7', 'Library and API documentation lookup.', 'npx', '["-y","@upstash/context7-mcp"]', '{}', 1, 1),
  ('playwright', 'Playwright', 'Browser automation and web testing tools.', 'npx', '["-y","@playwright/mcp"]', '{}', 1, 1);

-- Default CLI-MCP bindings.
INSERT INTO cli_mcp_bindings (cli_name, mcp_server_id) VALUES
  ('claude', 'context7'),
  ('codex',  'context7'),
  ('claude', 'playwright'),
  ('codex',  'playwright');
