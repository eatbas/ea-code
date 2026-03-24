# Test Coverage Implementation Plan

> Generated 2026-03-24 | Scope: `frontend/desktop/` (Rust + TypeScript) | Excludes: `hive-api/` (already tested)

## Current State

**Zero test files exist** across the entire desktop application. No `*.test.ts`, `*.spec.ts`, `*_test.rs`, or `tests/` directories. All testing infrastructure must be built from scratch.

---

## Priority Tiers

### Tier 1 — Pure Logic & Parsing (Highest ROI)

No mocking required. Pure functions with deterministic input/output. Start here.

#### Rust

| Module | Path | Lines | Key Functions | Risk |
|--------|------|-------|---------------|------|
| Plan Parser | `src-tauri/src/orchestrator/parsing/plan.rs` | ~80 | `parse_plan_audit_output`, `strip_legacy_verdict_prefix`, `strip_plan_tail_noise`, `looks_like_template_noise` | HIGH |
| Review Parser | `src-tauri/src/orchestrator/parsing/reviewer.rs` | ~160 | `parse_review_findings`, `normalise_findings`, verdict extraction, consensus marker parsing, dedup | HIGH |
| Skill Selection Parser | `src-tauri/src/orchestrator/skill_selection.rs` | ~123 | `parse_skill_selection_output` (JSON), `build_skill_selector_user`, `build_selected_skills_section` | MEDIUM |
| Pipeline Stage Serde | `src-tauri/src/models/pipeline.rs` | ~215 | Custom serialize/deserialize for 13 `PipelineStage` variants, `ExtraPlan(u8)` / `ExtraReviewer(u8)` roundtrip | HIGH |
| RunEvent Serde | `src-tauri/src/models/events.rs` | ~153 | All 8 `RunEvent` variants serialization, camelCase rename, optional fields | MEDIUM |
| Settings Defaults | `src-tauri/src/models/settings.rs` | ~425 | Default values, agent/model binding validation, extra slot arrays | MEDIUM |
| Storage Structs Serde | `src-tauri/src/models/storage.rs` | ~228 | `RunSummary`, `ReviewFindings`, `GitBaseline`, `ChatMessage` roundtrip | MEDIUM |
| Git Helpers | `src-tauri/src/git.rs` | ~75 | `is_git_repo`, `git_status`, `git_branch`, `workspace_info` | MEDIUM |

**Test approach:** `#[cfg(test)] mod tests` with `#[test]` (sync) for serde, string parsing. Collect real CLI output samples from Claude/Codex/Gemini as fixtures.

**Example test targets for `parsing/plan.rs`:**
- Input with `--- Improved Plan ---` marker extracts everything after
- Legacy verdict lines (`APPROVED:`, `REJECTED:`) stripped
- Tail noise (token counts, codex transcripts, markdown images) removed
- Template noise detection returns true for boilerplate
- Empty output falls back to `fallback_plan`

**Example test targets for `parsing/reviewer.rs`:**
- Markdown sections (`## blockers`, `## warnings`, `## nits`, `## tests`) parsed correctly
- Verdict extraction: explicit PASS/FAIL, fallback heuristic (blockers present = FAIL)
- Consensus markers (`2/3 agree`) parsed; low-consensus blockers demoted to warnings
- Placeholder items (`none`, `n/a`, `no issues`) filtered
- Bullet point extraction across varied markdown styles

#### TypeScript

| Module | Path | Lines | Key Functions | Risk |
|--------|------|-------|---------------|------|
| Status Helpers | `src/utils/statusHelpers.ts` | ~121 | `isActive`, `isTerminal`, `isRunInProgress`, `statusTone`, `statusToneClasses`, `statusInfo` | HIGH |
| Plan Parser | `src/utils/planParser.ts` | ~149 | `extractPlanOnly`, `stripPlanTailNoise`, `extractAfterLastPlanMarker`, `isLikelyTemplateNoise`, `resolvePlanText`, `resolveAuditedPlanText` | HIGH |
| CLI Parser | `src/utils/cliParser.ts` | ~46 | `tryParseJson`, `parseCliResult` (result + usage + cost + duration) | MEDIUM |
| Formatters | `src/utils/formatters.ts` | ~138 | `folderName`, `parseUtcTimestamp`, `formatTimestamp`, `formatCompactTimestamp`, `formatDuration`, `formatTokens`, `formatCost` | MEDIUM |
| Agent Settings | `src/utils/agentSettings.ts` | ~190 | `missingMinimumAgentLabels`, `hasMinimumAgentsConfigured`, `sanitiseAgentAssignmentsForEnabledModels`, `enabledModelsForBackend`, `sanitiseExtraSlots` | HIGH |
| Stage Model Labels | `src/utils/stageModelLabels.ts` | small | `getStageModelLabel` | LOW |

**Test approach:** Vitest (recommended) or Jest. No DOM, no mocking. Pure input/output.

**Example test targets for `statusHelpers.ts`:**
- `isActive("running")` = true, `isActive("completed")` = false
- `isTerminal("failed")` = true, `isTerminal("running")` = false
- `statusTone` maps every `PipelineStatus` value to the correct tone
- Unknown/undefined status handled gracefully

**Example test targets for `agentSettings.ts`:**
- 6 required roles detected when unset
- Model cleared when backend removed
- Model falls back to first enabled when current unavailable
- CSV model list parsed correctly
- Extra planner/reviewer slots sanitised

**Estimated tests:** 40–50 | **Effort:** ~2 weeks

---

### Tier 2 — Storage & Persistence

File I/O with atomic writes. Needs `tempfile` crate for isolated directories.

| Module | Path | Lines | Key Functions | Risk |
|--------|------|-------|---------------|------|
| Atomic Write | `src-tauri/src/storage/mod.rs` | ~200 | `atomic_write` (tmp → bak → final), `config_dir`, per-file mutex locks | CRITICAL |
| Run Lifecycle | `src-tauri/src/storage/runs/mod.rs` | ~322 | `create_run`, `update_run_summary`, `get_run`, `list_runs`, sequence numbering | CRITICAL |
| Event Log | `src-tauri/src/storage/runs/events.rs` | append | `append_event`, `get_events`, JSONL format, sequence continuity | HIGH |
| Projects | `src-tauri/src/storage/projects.rs` | ~211 | `add_project`, `read_projects`, `find_by_path`, 50-project cap, sort by recency | HIGH |
| Sessions | `src-tauri/src/storage/sessions.rs` | ~281 | `create_session`, `get_session`, `list_sessions`, `increment_run_count` | HIGH |
| Index | `src-tauri/src/storage/index.rs` | ~191 | `add_run_to_index`, `get_session_for_run`, rebuild on corruption | MEDIUM |
| Settings I/O | `src-tauri/src/storage/settings.rs` | small | `get_settings`, `save_settings` | MEDIUM |
| Skills I/O | `src-tauri/src/storage/skills.rs` | small | Skills CRUD | LOW |
| MCP Config | `src-tauri/src/storage/mcp.rs` | small | MCP server CRUD, CLI bindings | LOW |
| Migration | `src-tauri/src/storage/migration.rs` | varies | Flat → hierarchical layout migration | MEDIUM |
| Recovery | `src-tauri/src/storage/recovery.rs` | varies | Interrupted run detection, config dir migration | MEDIUM |

**Test approach:** `#[tokio::test]` with `tempfile::tempdir()`. Each test gets an isolated directory. Verify file contents after operations.

**Critical test scenarios for `atomic_write`:**
- Successful 3-step write: .tmp created → .bak created → final in place
- Partial write recovery: .tmp exists but final missing → recover from .bak
- Concurrent writes serialised by mutex
- Invalid path handling

**Critical test scenarios for `runs/mod.rs`:**
- Run directory structure created correctly
- `summary.json` contains correct initial state
- `RunStart` event appended on creation
- Summary updates are atomic (no partial writes)
- Sequence numbers monotonically increase
- Run listing returns correct order

**Critical test scenarios for `events.rs`:**
- Events appended in JSONL format (one JSON per line)
- Sequence numbers continuous (no gaps)
- All `RunEvent` variants serialise correctly
- Large event logs load without corruption

**Estimated tests:** 30–40 | **Effort:** ~2 weeks

---

### Tier 3 — Hooks & Commands

State management and IPC boundary. Requires Tauri invoke mocking.

#### TypeScript Hooks

| Hook | Path | Lines | What to Test | Risk |
|------|------|-------|-------------|------|
| `usePipelineEvents` | `src/hooks/usePipelineEvents.ts` | ~240 | Event subscription, run state creation, stage accumulation, log buffering, question handling, completion/error states, background run handling | CRITICAL |
| `usePipeline` | `src/hooks/usePipeline.ts` | ~196 | `startPipeline`, `pausePipeline`, `resumePipeline`, `cancelPipeline`, `answerQuestion`, artifact pruning on pause, `stageResetPredicate` | HIGH |
| `useHistory` | `src/hooks/useHistory.ts` | ~77 | Project/session/run loading, error handling, pagination | MEDIUM |
| `useSettings` | `src/hooks/useSettings.ts` | ~69 | Load on mount, persist on change, error recovery with defaults | MEDIUM |
| `useAppViewState` | `src/hooks/useAppViewState.ts` | varies | View transitions, session selection, navigation | MEDIUM |
| `useWorkspace` | `src/hooks/useWorkspace.ts` | small | Folder selection, validation | LOW |

**Test approach:** Vitest + `@testing-library/react` (`renderHook`). Mock `@tauri-apps/api` invoke/listen.

**Example test targets for `usePipelineEvents`:**
- `pipeline:started` event creates new run state with correct fields
- `pipeline:stage` events accumulate stages in order
- `pipeline:log` events buffer per stage
- `pipeline:artifact` events register artifacts
- `pipeline:question` event sets pending question state
- `pipeline:completed` event marks run terminal
- `pipeline:error` event marks run failed
- Background run not adopted when foreground run active

#### Rust Commands

| Command | Path | Lines | What to Test | Risk |
|---------|------|-------|-------------|------|
| `run_pipeline` | `src-tauri/src/commands/pipeline.rs` | ~209 | Request validation, flag/channel setup in AppState, minimum agent check, async spawn | HIGH |
| `cancel_pipeline` | same | small | Cancel flag set, answer channel signalled | HIGH |
| `pause_pipeline` | same | small | Pause flag set | MEDIUM |
| `resume_pipeline` | same | small | Pause flag cleared | MEDIUM |
| `answer_pipeline_question` | same | small | Answer sent through channel | MEDIUM |
| History commands | `src-tauri/src/commands/history.rs` | varies | Project/session/run listing, deletion, session detail | HIGH |

**Test approach:** `#[tokio::test]` with mock `AppState`. No real Tauri app handle needed for unit tests of flag/channel logic.

**Estimated tests:** 20–30 | **Effort:** ~1.5 weeks

---

### Tier 4 — Integration Tests

Full pipeline flow with mocked agents. Verifies orchestration correctness end-to-end.

| Scenario | Modules Covered | Description |
|----------|-----------------|-------------|
| Single-iteration success | orchestrator, storage, events | Prompt → Plan → Code → Review (no blockers) → Judge COMPLETE |
| Multi-iteration fix loop | orchestrator, iteration, review, judge | Judge returns NOT_COMPLETE → second iteration → COMPLETE |
| Plan rejection and revision | plan_gate, iteration_planning | Plan audit REJECTED → user revises → re-plan → APPROVED |
| Cancellation mid-pipeline | orchestrator, commands, stages | Cancel flag set during code stage → pipeline terminates cleanly |
| Pause and resume | orchestrator, commands | Pause during review → resume → pipeline continues |
| Parallel reviewers | iteration_review | 3 reviewers execute, findings merged, consensus applied |
| Session memory carryover | session_memory, storage | Second run builds context from first run's history |
| DirectTask mode | pipeline/direct_task | Single-agent bypass of full pipeline |
| Event sequence verification | events, runs | Verify emitted event sequence matches expected state machine |
| Crash recovery | storage/recovery | Interrupted run detected and marked on restart |

**Test approach:** Full `#[tokio::test]` with:
- Mock storage layer (in-memory or tempdir)
- Mock agent dispatch (return canned output instead of spawning CLI)
- Real orchestrator logic
- Verify emitted events and final storage state

**Estimated tests:** 10–20 | **Effort:** ~2–3 weeks

---

## IPC Type Contract Tests

Frontend TypeScript types must stay in sync with Rust serde output. These are cross-layer tests.

| Concern | Rust Source | TS Source | What to Verify |
|---------|------------|-----------|----------------|
| PipelineStage enum values | `models/pipeline.rs` | `types/pipeline.ts` | All 13 variants serialise to identical strings |
| PipelineStatus values | `models/pipeline.rs` | `types/pipeline.ts` | Status string literals match |
| RunEvent variants | `models/events.rs` | `types/events.ts` | Event type tags and payload shapes match |
| AgentBackend values | `models/agents.rs` | `types/agents.ts` | Backend names match |
| JudgeVerdict values | `models/pipeline.rs` | `types/pipeline.ts` | Verdict strings match |
| StageEndStatus | `models/events.rs` | `types/events.ts` | Status values match |
| AppSettings fields | `models/settings.rs` | `types/settings.ts` | camelCase field names match |

**Test approach:** Generate JSON fixtures from Rust tests, validate against TypeScript interfaces using Vitest schema validation (e.g. zod or manual checks).

---

## Testing Infrastructure Setup

### Rust (`src-tauri/`)

```toml
# Already available in Cargo.toml via #[cfg(test)]
[dev-dependencies]
tempfile = "3"       # Isolated temp directories for storage tests
tokio = { features = ["test-util", "macros"] }  # #[tokio::test]
serde_json = "1"     # Fixture loading
```

**File structure:**
```
src-tauri/src/
  orchestrator/
    parsing/
      plan.rs          # Add #[cfg(test)] mod tests at bottom
      reviewer.rs      # Add #[cfg(test)] mod tests at bottom
  models/
    pipeline.rs        # Add #[cfg(test)] mod tests at bottom
    events.rs          # Add #[cfg(test)] mod tests at bottom
  storage/
    mod.rs             # Add #[cfg(test)] mod tests at bottom
    runs/mod.rs        # Add #[cfg(test)] mod tests at bottom
  tests/               # Integration tests (separate binary)
    pipeline_flow.rs
    storage_integration.rs
```

**Run:** `cd frontend/desktop/src-tauri && cargo test`

### TypeScript (`src/`)

```json
// vitest.config.ts (new file)
{
  "test": {
    "environment": "node",
    "include": ["src/**/*.test.ts"],
    "coverage": { "provider": "v8" }
  }
}
```

**File structure:**
```
src/
  utils/
    __tests__/
      statusHelpers.test.ts
      planParser.test.ts
      cliParser.test.ts
      formatters.test.ts
      agentSettings.test.ts
  hooks/
    __tests__/
      usePipeline.test.ts
      usePipelineEvents.test.ts
  __mocks__/
    @tauri-apps/
      api.ts           # Mock invoke and listen
```

**Run:** `cd frontend/desktop && npx vitest run`

---

## Recommended Execution Order

```
Phase 1 (Week 1-2): Tier 1 — Pure logic and parsing
  ├── Rust: parsing/plan.rs, parsing/reviewer.rs, models/pipeline.rs serde
  ├── TS: statusHelpers, planParser, formatters, agentSettings, cliParser
  └── Setup: Vitest config, Rust test infrastructure

Phase 2 (Week 3-4): Tier 2 — Storage persistence
  ├── Rust: atomic_write, runs lifecycle, events JSONL, projects, sessions
  └── Add tempfile dev-dependency

Phase 3 (Week 5): Tier 3 — Hooks and commands
  ├── TS: usePipelineEvents, usePipeline (with Tauri mocks)
  └── Rust: pipeline command validation, flag/channel logic

Phase 4 (Week 6-7): Tier 4 — Integration
  ├── Full pipeline flow with mock agents
  ├── Cancellation/pause scenarios
  └── IPC type contract validation
```

---

## Risk Summary

| Risk Level | Count | Examples |
|------------|-------|---------|
| CRITICAL | 6 | Atomic writes, run persistence, pipeline orchestration, event subscriptions, iteration loop, IPC types |
| HIGH | 10 | Plan/review parsing, status helpers, agent dispatch, pipeline commands, agent settings validation |
| MEDIUM | 8 | Session memory, skill selection, formatters, CLI parser, index, settings I/O, git helpers |
| LOW | 4 | Skills I/O, MCP config, workspace hook, stage model labels |

**Total modules requiring tests:** 28 Rust + 11 TypeScript = **39 modules**
**Estimated total tests:** ~120–140
**Estimated total effort:** ~7 weeks (1 engineer)
