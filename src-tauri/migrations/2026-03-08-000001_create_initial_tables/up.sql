-- Initial schema for EA Code persistence layer.
-- Settings, projects, sessions, runs, iterations, stages, logs, artefacts, questions.

PRAGMA foreign_keys = ON;

-- Settings: single-row config table (replaces settings.json)
CREATE TABLE settings (
    id                  INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    claude_path         TEXT NOT NULL DEFAULT 'claude',
    codex_path          TEXT NOT NULL DEFAULT 'codex',
    gemini_path         TEXT NOT NULL DEFAULT 'gemini',
    generator_agent     TEXT NOT NULL DEFAULT 'claude',
    reviewer_agent      TEXT NOT NULL DEFAULT 'codex',
    fixer_agent         TEXT NOT NULL DEFAULT 'claude',
    final_judge_agent   TEXT NOT NULL DEFAULT 'codex',
    max_iterations      INTEGER NOT NULL DEFAULT 3,
    require_git         BOOLEAN NOT NULL DEFAULT 1,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO settings (id) VALUES (1);

-- Projects: workspace bookmarks with git metadata
CREATE TABLE projects (
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    path        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    is_git_repo BOOLEAN NOT NULL DEFAULT 0,
    branch      TEXT,
    last_opened TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_projects_last_opened ON projects(last_opened DESC);

-- Sessions: conversation threads (a "chat" in the sidebar)
CREATE TABLE sessions (
    id          TEXT NOT NULL PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title       TEXT NOT NULL DEFAULT 'New Session',
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_sessions_project ON sessions(project_id, updated_at DESC);

-- Runs: individual pipeline executions within a session
CREATE TABLE runs (
    id              TEXT NOT NULL PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    prompt          TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'running',
    max_iterations  INTEGER NOT NULL DEFAULT 3,
    final_verdict   TEXT,
    error           TEXT,
    started_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMP
);

CREATE INDEX idx_runs_session ON runs(session_id, started_at ASC);

-- Iterations: each loop of the self-improving pipeline
CREATE TABLE iterations (
    id              INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    number          INTEGER NOT NULL,
    verdict         TEXT,
    judge_reasoning TEXT,
    UNIQUE(run_id, number)
);

-- Stages: individual pipeline stages within an iteration
CREATE TABLE stages (
    id              INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    iteration_id    INTEGER NOT NULL REFERENCES iterations(id) ON DELETE CASCADE,
    stage           TEXT NOT NULL,
    status          TEXT NOT NULL,
    output          TEXT NOT NULL DEFAULT '',
    duration_ms     INTEGER NOT NULL DEFAULT 0,
    error           TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_stages_iteration ON stages(iteration_id);

-- Logs: streaming CLI output lines
CREATE TABLE logs (
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    run_id      TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    stage       TEXT NOT NULL,
    line        TEXT NOT NULL,
    stream      TEXT NOT NULL DEFAULT 'stdout',
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_logs_run ON logs(run_id, created_at);

-- Artefacts: diffs, reviews, validation output, judge reasoning
CREATE TABLE artifacts (
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    run_id      TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    iteration   INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    content     TEXT NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_artifacts_run ON artifacts(run_id, iteration);

-- Questions: user Q&A during pipeline pauses
CREATE TABLE questions (
    id              TEXT NOT NULL PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    stage           TEXT NOT NULL,
    iteration       INTEGER NOT NULL,
    question_text   TEXT NOT NULL,
    agent_output    TEXT NOT NULL DEFAULT '',
    optional        BOOLEAN NOT NULL DEFAULT 0,
    answer          TEXT,
    skipped         BOOLEAN NOT NULL DEFAULT 0,
    asked_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    answered_at     TIMESTAMP
);

CREATE INDEX idx_questions_run ON questions(run_id);
