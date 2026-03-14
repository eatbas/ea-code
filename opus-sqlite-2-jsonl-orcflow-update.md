# SQLite to JSONL Migration & Orchestrator Flow Update

> Final consolidated plan. Incorporates Opus analysis + Codex review feedback.
> Decision: full SQLite removal, not partial.

---

## Background

EA Code currently uses SQLite (Diesel ORM) for all persistence — settings, projects, sessions, runs, iterations, stages, artifacts, skills, MCP config, and questions. After analysis, we determined:

- The inter-stage communication during a pipeline run is already in-memory (`IterationContext` struct). The DB was never the communication channel between stages — it only persisted results after the fact.
- Storing 50K of raw agent output per stage, diffs, and iteration-level detail is unnecessary bloat.
- Agents (Coder, Reviewer, Fixer, Judge) already inspect the git repo themselves — stored diffs are redundant.
- A file-based approach (JSON + JSONL) is simpler, human-readable, debuggable, and aligns with how other AI coding tools (Claude Code, Cursor, etc.) handle persistence.

---

## What We Decided

### Data to REMOVE entirely

| Data | Reason |
|------|--------|
| **Git diffs as artifacts** | Agents read git themselves. Git is the source of truth. |
| **Stage raw outputs (50K each)** | Coder's text output is just a CLI log — nobody reads it. Same for most stage outputs. |
| **Iteration-level detail** (plan_revision_count, plan_approval tracking) | Over-engineered. Flatten into the JSONL event log. |
| **fix_output passed to Judge** | Judge inspects git itself, doesn't need the Fixer's narration. |
| **Full raw review_output passed to Judge** | Replace with compact structured review findings (see below). |

### Data to KEEP

| Data | New Storage | Format |
|------|-------------|--------|
| App settings | `settings.json` | Single JSON object |
| Recent projects | `projects.json` | JSON array |
| Skills catalogue | `skills/<id>.json` | One file per skill |
| MCP servers + CLI bindings | `mcp.json` | Single JSON with servers + bindings |
| Session metadata | `sessions/<id>/session.json` | JSON object (rich — see below) |
| Run summary | `sessions/<id>/runs/<rid>/summary.json` | JSON object for fast reads |
| Run event log | `sessions/<id>/runs/<rid>/events.jsonl` | Append-only JSONL |
| Executive summary | Inside `summary.json` | Part of run summary |
| Questions asked/answered | Inside `events.jsonl` as question events | Part of event log |
| Files changed per run | Inside `summary.json` | List of paths |

### Orchestrator flow changes

| Change | Detail |
|--------|--------|
| **Replace raw review_output to Judge with compact structured findings** | Instead of passing the full reviewer text, pass a compact summary: unresolved blockers, warnings, test gaps, explicit failure conditions. This keeps useful signal without transcript bloat. |
| **Stop passing fix_output to Judge** | Judge reads git diff itself. Remove `fix_out` from `build_judge_user()`. |
| **Keep passing full review_output to Fixer** | Fixer NEEDS this — it tells the Fixer what BLOCKERs/WARNINGs to address. |
| **Remove diff capture stages** | `DiffAfterCoder` and `DiffAfterCodeFixer` stages removed as stored artifacts. |
| **Plain text prompts** | Continue using plain text piped via stdin. No markdown files needed. |

---

## Target File Structure

```
~/.config/ea-code/
  settings.json
  projects.json
  mcp.json
  skills/
    <skill-id>.json
  sessions/
    <session-id>/
      session.json
      runs/
        <run-id>/
          summary.json
          events.jsonl
```

### Why per-run directory (not flat JSONL)

Codex correctly identified that a single JSONL file per run forces scanning for fast reads. The per-run directory with `summary.json` + `events.jsonl` gives:

- **`summary.json`** — fast reads for history views, sidebar, session listing. Written once at run completion (or updated on crash recovery).
- **`events.jsonl`** — append-only, debuggable, human-readable event stream. Only read when the user drills into a specific run's detail view.
- Partial or failed runs are easier to recover from.
- Future extensions don't force repeated JSONL scans.

---

## Rich Session Metadata

`session.json` must contain enough for the sidebar and session list without scanning run logs:

```json
{
  "id": "session-uuid",
  "title": "JWT Authentication",
  "projectPath": "/home/user/my-project",
  "runCount": 3,
  "lastPrompt": "add rate limiting to the auth endpoints",
  "lastStatus": "completed",
  "lastVerdict": "complete",
  "createdAt": "2026-03-15T09:00:00Z",
  "updatedAt": "2026-03-15T10:05:00Z"
}
```

Updated every time a run completes (or fails/cancels). This avoids scanning run directories for listing.

---

## Run Summary Format

`summary.json` — written/updated at run end. Also serves as the live run snapshot during execution.

```json
{
  "schemaVersion": 1,
  "id": "run-uuid",
  "sessionId": "session-uuid",
  "prompt": "add JWT auth to the API",
  "enhancedPrompt": "Implement JWT-based authentication...",
  "status": "completed",
  "finalVerdict": "complete",
  "currentStage": null,
  "currentIteration": null,
  "totalIterations": 2,
  "maxIterations": 3,
  "executiveSummary": "Added JWT auth with RS256 signing...",
  "filesChanged": ["src/auth.rs", "src/routes/mod.rs"],
  "error": null,
  "startedAt": "2026-03-15T10:00:00Z",
  "completedAt": "2026-03-15T10:05:00Z"
}
```

**Live run state**: During an active run, `summary.json` is updated periodically with `currentStage`, `currentIteration`, and `status: "running"`. This survives app reloads — if the app restarts, it can read the summary to know a run was in progress and which stage it was on.

---

## JSONL Event Log Format

`events.jsonl` — append-only. Every line has a schema version, sequence number, and timestamp.

```jsonl
{"v":1,"seq":1,"ts":"2026-03-15T10:00:00Z","type":"run_start","prompt":"add JWT auth","maxIterations":3}
{"v":1,"seq":2,"ts":"2026-03-15T10:00:03Z","type":"stage_start","stage":"prompt_enhance","iteration":1}
{"v":1,"seq":3,"ts":"2026-03-15T10:00:07Z","type":"stage_end","stage":"prompt_enhance","iteration":1,"status":"completed","durationMs":3200}
{"v":1,"seq":4,"ts":"2026-03-15T10:00:07Z","type":"stage_start","stage":"plan","iteration":1}
{"v":1,"seq":5,"ts":"2026-03-15T10:00:14Z","type":"stage_end","stage":"plan","iteration":1,"status":"completed","durationMs":7100}
{"v":1,"seq":6,"ts":"2026-03-15T10:00:14Z","type":"stage_start","stage":"plan_audit","iteration":1}
{"v":1,"seq":7,"ts":"2026-03-15T10:00:19Z","type":"stage_end","stage":"plan_audit","iteration":1,"status":"completed","auditVerdict":"APPROVED","durationMs":4800}
{"v":1,"seq":8,"ts":"2026-03-15T10:00:19Z","type":"stage_start","stage":"coder","iteration":1}
{"v":1,"seq":9,"ts":"2026-03-15T10:00:34Z","type":"stage_end","stage":"coder","iteration":1,"status":"completed","durationMs":15000}
{"v":1,"seq":10,"ts":"2026-03-15T10:00:34Z","type":"stage_start","stage":"reviewer","iteration":1}
{"v":1,"seq":11,"ts":"2026-03-15T10:00:39Z","type":"stage_end","stage":"reviewer","iteration":1,"status":"completed","durationMs":5000}
{"v":1,"seq":12,"ts":"2026-03-15T10:00:39Z","type":"stage_start","stage":"fixer","iteration":1}
{"v":1,"seq":13,"ts":"2026-03-15T10:00:47Z","type":"stage_end","stage":"fixer","iteration":1,"status":"completed","durationMs":8000}
{"v":1,"seq":14,"ts":"2026-03-15T10:00:47Z","type":"stage_start","stage":"judge","iteration":1}
{"v":1,"seq":15,"ts":"2026-03-15T10:00:51Z","type":"stage_end","stage":"judge","iteration":1,"status":"completed","verdict":"not_complete","durationMs":4000}
{"v":1,"seq":16,"ts":"2026-03-15T10:00:51Z","type":"iteration_end","iteration":1,"verdict":"not_complete"}
{"v":1,"seq":17,"ts":"2026-03-15T10:00:52Z","type":"question","stage":"coder","iteration":1,"question":"Should I use RS256 or HS256?","answer":"RS256"}
{"v":1,"seq":18,"ts":"2026-03-15T10:01:00Z","type":"stage_start","stage":"coder","iteration":2}
{"v":1,"seq":19,"ts":"2026-03-15T10:04:50Z","type":"stage_end","stage":"judge","iteration":2,"status":"completed","verdict":"complete","durationMs":3500}
{"v":1,"seq":20,"ts":"2026-03-15T10:04:50Z","type":"iteration_end","iteration":2,"verdict":"complete"}
{"v":1,"seq":21,"ts":"2026-03-15T10:05:00Z","type":"run_end","status":"completed","verdict":"complete"}
```

### Event types

| Event | Required fields | Purpose |
|-------|----------------|---------|
| `run_start` | prompt, maxIterations | Marks the beginning of a pipeline run |
| `stage_start` | stage, iteration | A stage begins execution |
| `stage_end` | stage, iteration, status, durationMs | A stage finishes (completed/failed/skipped) |
| `iteration_end` | iteration, verdict | An iteration loop completes with judge verdict |
| `question` | stage, iteration, question, answer | User answered a question during the run |
| `run_end` | status, verdict | Terminal event — run completed, failed, or cancelled |

All events carry `v` (schema version), `seq` (monotonic sequence), `ts` (RFC 3339 timestamp).

### Terminal events

Every run MUST end with exactly one terminal event:
- `{"type":"run_end","status":"completed",...}` — normal completion
- `{"type":"run_end","status":"failed","error":"...",...}` — stage failure
- `{"type":"run_end","status":"cancelled",...}` — user cancelled

If no terminal event exists, the run is considered crashed (see crash recovery below).

---

## Crash Recovery Rules

### Detection

On app startup, scan active sessions for runs where:
- `summary.json` has `"status": "running"` but no process is active
- `events.jsonl` exists but has no `run_end` terminal event

These are crashed runs.

### Recovery

1. Read the last valid line in `events.jsonl` to determine where the run stopped.
2. Update `summary.json`:
   - Set `status` to `"crashed"`
   - Set `currentStage` and `currentIteration` from the last event
   - Set `error` to `"Run interrupted — app closed or crashed during execution"`
   - Set `completedAt` to the timestamp of the last event
3. Append a synthetic terminal event to `events.jsonl`:
   ```jsonl
   {"v":1,"seq":N,"ts":"...","type":"run_end","status":"crashed","error":"recovered on startup","recoveredAt":"..."}
   ```
4. Update `session.json` metadata (lastStatus, updatedAt).

### Malformed line handling

- If a JSONL line fails to parse, skip it and continue reading.
- Log a warning but do not fail the entire read operation.
- The `seq` field allows detecting gaps from skipped lines.

### Partial write handling

- Use atomic writes for `summary.json` and `session.json`: write to a `.tmp` file, then rename. This prevents half-written JSON.
- JSONL appends are naturally safe — a partial last line is simply a malformed line and gets skipped on read.

---

## Compact Structured Review for Judge

Instead of passing the full raw reviewer output to the Judge, extract a compact structured summary. The Reviewer already outputs in a structured format (BLOCKER/WARNING/NIT sections). Parse this into a compact block:

```
--- Review Findings ---
BLOCKERS: 2
  - Missing input validation on POST /auth/login (line 45 of auth.rs)
  - No error handling for expired refresh tokens
WARNINGS: 1
  - Token expiry hardcoded to 3600s, should be configurable
TESTS: not run
VERDICT: FAIL
```

This gives the Judge:
- Whether there are unresolved blockers (most important signal)
- Specific issues to verify in the code
- Whether tests were run
- The reviewer's overall verdict

Without the full transcript (which can be 2000+ tokens of narration the Judge doesn't need).

The Fixer still receives the FULL review output — it needs the complete detail to know exactly what to fix and where.

---

## Run Start Baseline

To reliably determine which files changed during a run, capture a git baseline at run start:

1. At `run_start`, record the current `HEAD` commit SHA and whether the working tree is dirty.
2. Store this in `summary.json` as `gitBaseline`:
   ```json
   {
     "gitBaseline": {
       "commitSha": "abc123",
       "hadUnstagedChanges": true
     }
   }
   ```
3. At run end, compute files changed by comparing against the baseline:
   - If the tree was clean at start: `git diff --name-only <baseline-sha>..HEAD`
   - If the tree was dirty at start: compare the working tree diff lists (less reliable, but noted)
4. If the baseline comparison is unreliable (branch moved, etc.), fall back to listing files changed in the most recent commits made during the run window.

---

## Session Context for Next Prompt

When the user enters a new prompt in the same session, build context by reading `summary.json` from previous runs in that session. Extract:

- What was the prompt
- What was the verdict (complete/not complete)
- The executive summary
- Files changed
- Key questions and answers

This replaces the current `session_memory.rs` which queries the DB. Since `summary.json` is a fast read (no JSONL scanning), building session context is efficient.

---

## Retention Cleanup

Walk `sessions/*/runs/*/summary.json`:
- Parse `completedAt` (or `startedAt` if no completion timestamp).
- For runs with `status` of `completed`, `failed`, `cancelled`, or `crashed` — delete the run directory if older than the configured retention days.
- For runs with no `summary.json` or unparseable timestamps — treat as crashed, apply crash recovery first, then evaluate retention.
- Delete empty session directories after cleanup.
- Delete empty `runs/` directories.

---

## Implementation Plan

### Phase 0: Define the storage contract

1. **Define the Rust serde structs** for all file models: `SettingsFile`, `ProjectEntry`, `SkillFile`, `McpConfigFile`, `SessionMeta`, `RunSummary`, `RunEvent` (enum with all event types). Add `schema_version` fields to `RunSummary` and `RunEvent`.

2. **Define the matching TypeScript types** in `frontend/desktop/src/types/`. These must mirror the Rust structs exactly.

3. **Define terminal event rules** — every run must end with a `run_end` event. Define the `crashed` recovery status. Define malformed-line skip behaviour.

4. **Define the compact review findings struct** — `ReviewFindings { blockers: Vec<String>, warnings: Vec<String>, testsRun: bool, verdict: String }`. This is what gets passed to the Judge instead of raw output.

### Phase 1: Create the file-based storage layer

5. **Create a new `storage` module** in `src-tauri/src/` to replace `db/`. Submodules: `settings.rs`, `projects.rs`, `skills.rs`, `mcp.rs`, `sessions.rs`, `runs.rs`, `cleanup.rs`, `recovery.rs`.

6. **Implement settings storage** — read/write `settings.json` with atomic writes (write `.tmp`, rename). On first launch, if no `settings.json` exists but the old SQLite DB does, migrate from DB.

7. **Implement projects storage** — read/write `projects.json`. Array sorted by `lastOpened` descending. Cap at 50 recent projects.

8. **Implement skills storage** — CRUD on `skills/<id>.json` files. Listing = glob + read each.

9. **Implement MCP storage** — read/write `mcp.json` with built-in server sync logic.

10. **Implement session storage** — create session directories, read/write `session.json` with rich metadata (runCount, lastPrompt, lastStatus, lastVerdict). Listing = read all `sessions/*/session.json`, sort by `updatedAt`.

11. **Implement run storage** — create run directories, write `summary.json` (atomic), append to `events.jsonl`. Helpers: `create_run()`, `append_event()`, `update_summary()`, `read_summary()`, `read_events()`, `list_runs()`.

12. **Implement crash recovery** — on startup, scan for runs with `status: "running"` and no active process. Apply recovery rules. Append synthetic `run_end` event. Update `summary.json` and `session.json`.

13. **Implement retention cleanup** — walk run directories, parse timestamps, delete old run directories, clean up empty session directories.

### Phase 2: Update the orchestrator

14. **Replace all `db::` calls in the pipeline** with `storage::` equivalents. The pipeline currently calls `db::runs::insert`, `db::runs::insert_iteration`, `db::runs::update_iteration_verdict`, `db::runs::complete`, `db::sessions::touch`, and `emit_artifact`. Replace with: `storage::runs::create_run()`, `storage::runs::append_event()`, `storage::runs::update_summary()`, `storage::sessions::update_meta()`.

15. **Remove the diff capture stages** — delete `DiffAfterCoder` and `DiffAfterCodeFixer` from `iteration.rs` and `iteration_review.rs`. Remove `execute_diff_stage`. Remove diff artifact emissions.

16. **Add compact review findings parser** — after the Reviewer stage completes, parse its structured output (BLOCKER/WARNING/NIT sections, PASS/FAIL verdict) into a `ReviewFindings` struct. Pass this compact struct to the Judge prompt instead of raw text.

17. **Update the Judge prompt** — `build_judge_user()` receives `ReviewFindings` (compact) instead of `rev_out` (raw). Remove `fix_out` entirely. Update `prompts/judge.rs` to format the compact findings block and emphasise git inspection as the primary evaluation method.

18. **Update session memory** — rewrite `session_memory.rs` to read `summary.json` files from the session's run directories instead of querying the DB.

19. **Capture git baseline at run start** — record `HEAD` SHA and dirty state in `summary.json`. At run end, compute files changed from baseline and include in summary.

20. **Update live run state** — during execution, periodically update `summary.json` with `currentStage`, `currentIteration`, `status: "running"`. This survives app reloads.

### Phase 3: Update the Tauri commands (frontend API)

21. **Rewrite `commands/history.rs`** — `list_projects`, `list_sessions`, `get_session_detail`, `create_session`, `delete_session`, `get_run_detail` now read from files. Session listing reads `session.json` files (fast, no run scanning). Run detail reads `summary.json` + optionally `events.jsonl` for the timeline view.

22. **Rewrite `commands/settings.rs`** — `get_settings` and `save_settings` read/write `settings.json`.

23. **Rewrite `commands/skills.rs`** — CRUD on `skills/*.json` files.

24. **Rewrite `commands/mcp.rs`** — CRUD on `mcp.json`.

25. **Update `commands/pipeline.rs`** — pass storage handle instead of `DbPool` to the orchestrator.

### Phase 4: Remove SQLite dependencies

26. **Remove Diesel and SQLite dependencies** from `Cargo.toml` — delete `diesel`, `diesel_migrations`, `r2d2`, `libsqlite3-sys`.

27. **Delete the `db/` module entirely** — all submodules.

28. **Delete `schema.rs`** and the `migrations/` directory.

29. **Remove `DbPool` from all function signatures** — follow compiler errors through orchestrator, commands, agents, stages.

30. **Update `main.rs`** — remove `init_db()`, remove `DbPool` from Tauri managed state. Add storage initialisation (ensure directories exist, run crash recovery, run retention cleanup, sync built-in MCP servers).

31. **One-time DB migration** — on first launch, if `ea-code.db` exists but `settings.json` does not, import all data from SQLite into the new file structure. Leave the `.db` file in place as backup.

### Phase 5: Frontend updates

32. **Update TypeScript types** — remove DB-specific types (iteration detail, stage detail with raw output, artifact kinds). Add types matching the new file models: `SessionMeta`, `RunSummary`, `RunEvent`.

33. **Update history/chat views** — session list now powered by `SessionMeta` (fast, no scanning). Run detail shows: prompt, status, verdict, summary, files changed, questions, and timing per stage (from events).

34. **Remove artifact viewers** — no more diff display or raw stage output display.

35. **Verify all Tauri `invoke()` calls** match updated command signatures.

### Phase 6: Testing and cleanup

36. **End-to-end pipeline test** — run a prompt through all stages, verify `events.jsonl` and `summary.json` are created correctly, verify session context carries to next prompt.

37. **Crash recovery test** — kill the app mid-run, restart, verify recovery marks the run as crashed and the UI shows it correctly.

38. **Settings migration test** — verify existing users' settings are imported from SQLite on first launch.

39. **Retention cleanup test** — verify old run directories are deleted correctly.

40. **Dead code removal** — grep for remaining references to `DbPool`, `diesel`, `schema::`, old artifact kinds.

41. **`cargo check`** — backend compiles cleanly.

42. **`npx tsc --noEmit`** — frontend compiles cleanly.

---

## Risk Notes

- **Data loss for existing users** — the one-time migration (step 31) must be thorough. The old `.db` file is kept as backup, never deleted automatically.
- **Session listing performance** — reading `session.json` files will be slower than a SQL query. For most users (< 100 sessions) this is negligible. If it becomes a problem later, add a lightweight `sessions/index.json` cache.
- **Concurrent writes** — use atomic writes (write `.tmp`, rename) for `summary.json` and `session.json`. JSONL appends are naturally safe. Only one pipeline run should be active per session at a time (existing constraint).
- **File system limits** — thousands of run directories in a single session is unlikely but possible. Retention cleanup prevents unbounded growth.
