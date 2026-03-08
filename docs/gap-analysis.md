# EA Orchestrator → EA Code: Feature Gap Analysis

## Context

The original **eaOrch** was a VS Code extension (~13,800 LOC TypeScript) orchestrating multiple AI CLIs (Claude, Codex, Gemini, Kimi, Copilot, OpenCode) in a self-improving dev loop. It has been partially ported to **ea-code**, a Tauri v2 desktop application (Rust backend + React frontend).

This document tracks what has been ported, what remains, and the implementation roadmap.

---

## Phase 1: Prompt Engineering Upgrade — COMPLETED

All items in Phase 1 have been implemented and merged to `main`.

| Item | Status |
|------|--------|
| Port eaOrch v2.5.0 prompts to Rust | Done |
| Iteration awareness in all system prompts (`"iteration N of M"`) | Done |
| 5-criteria judge rubric (3 REQUIRED, 2 RECOMMENDED) with `[x]/[ ]` checklist | Done |
| BLOCKER/WARNING/NIT severity classification in reviewer | Done |
| Structured iteration handoff JSON parsing and injection | Done |
| Final-iteration pragmatic standard (judge relaxes on minor NITs) | Done |
| Token budget guidance per role | Done |
| Scope guardrails ("only necessary changes", "preserve approach") | Done |
| Context7 MCP instructions in all prompts | Done |
| 3-tier verdict parsing (exact → checklist heuristic → keyword fail-safe) | Done |
| Progress awareness (prior judge output truncated to 3000 chars) | Done |
| Split monolithic `orchestrator.rs` (1,638 lines) into 12 modules (all under 300 lines) | Done |

### Module Structure (post Phase 1)

```
src-tauri/src/orchestrator/
├── mod.rs                  (20 lines)   Module declarations
├── pipeline.rs             (124 lines)  Main run_pipeline loop
├── iteration.rs            (235 lines)  Single iteration + planning stages
├── iteration_review.rs     (178 lines)  Review/fix/judge sub-stages
├── run_setup.rs            (205 lines)  IterationContext, executive summary, final status
├── helpers.rs              (268 lines)  Dispatch, events, cancellation, persistence
├── stages.rs               (171 lines)  Stage execution functions
├── parsing.rs              (191 lines)  Verdict/audit parsing (3-tier strategy)
└── prompts/
    ├── mod.rs              (203 lines)  PromptMeta, IterationHandoff, handoff parsing
    ├── planning.rs         (177 lines)  Enhancer, planner, plan auditor prompts
    ├── execution.rs        (240 lines)  Generator, reviewer, fixer prompts
    └── judge.rs            (196 lines)  Judge prompt with rubric/progress/final guidance
```

---

## Phase 2: Pipeline Enhancements — REMAINING

### 2.1 Plan User Gate (HIGH)

eaOrch has an interactive plan approval flow. ea-code has `answer_pipeline_question()` for questions but no dedicated plan gate.

**What is needed:**
- Pipeline pauses after planning stage
- User can: **approve**, **request revisions** (up to `maxPlanRevisions`), or **skip**
- Auto-proceed timeout (`planResponseTimeoutSec`, default 45s)
- Revision loop: user feedback → planner re-plans → user re-reviews
- Up to `maxPlanRevisions` (default 3) revision rounds

**Files to modify:**
- `src-tauri/src/orchestrator/iteration.rs` — add plan gate logic after planning stages
- `src-tauri/src/models.rs` — add `PlanGateAction` enum (Approve/Revise/Skip)
- `src-tauri/src/commands.rs` — add `answer_plan_gate()` command
- `src/types/index.ts` — frontend types for plan gate
- `src/components/` — plan approval dialog UI

**New settings:**
- `planResponseTimeoutSec` (default: 45)
- `maxPlanRevisions` (default: 3)

---

### 2.2 Retry with Prompt Augmentation (MEDIUM)

eaOrch has `runWithRetry()` that retries failed agent calls once with a hint.

**What is needed:**
- 1 retry per agent call on failure
- On retry, append `"PREVIOUS ATTEMPT FAILED: {error_hint}"` to the prompt
- Never retry on cancellation (AbortError)

**Files to modify:**
- `src-tauri/src/orchestrator/stages.rs` — wrap `execute_agent_stage` with retry logic
- `src-tauri/src/agents/base.rs` — optional: retry-aware `AgentInput`

---

### 2.3 Token-Optimised Prompt Mode (MEDIUM)

eaOrch has dual prompt modes to reduce token usage on large codebases.

**What is needed:**
- New setting: `tokenOptimizedPrompts` (default: `false`)
- When enabled:
  - Reviewer/Judge instructed to run `git diff` locally instead of receiving inline diff
  - Handoff JSON replaces full judge output for next iteration
  - Significantly reduces token usage on large codebases

**Files to modify:**
- `src-tauri/src/orchestrator/prompts/execution.rs` — conditional reviewer prompt
- `src-tauri/src/orchestrator/prompts/judge.rs` — conditional judge prompt
- `src-tauri/src/orchestrator/iteration.rs` — conditional diff embedding
- `src-tauri/src/models.rs` — add `token_optimized_prompts` to `AppSettings`

---

## Phase 3: Skills System — REMAINING (Entirely Missing)

eaOrch has a complete skills system; ea-code has nothing.

### 3.1 Skill Data Model

**What is needed:**
- DB table for skills: `id`, `name`, `description`, `instructions`, `tags`, `created_at`, `updated_at`
- Rust model: `Skill` struct with serde
- Frontend type in `src/types/index.ts`

**Files to create/modify:**
- `src-tauri/migrations/` — new migration for `skills` table
- `src-tauri/src/schema.rs` — Diesel schema update
- `src-tauri/src/db/skills.rs` — CRUD queries
- `src-tauri/src/models.rs` — `Skill` struct

### 3.2 Skill CRUD

**What is needed:**
- Tauri commands: `create_skill`, `update_skill`, `delete_skill`, `list_skills`, `get_skill`
- Frontend hooks: `useSkills()`
- Settings UI for skill management

### 3.3 Skill Selection

**What is needed:**
- AI agent selects 0-3 relevant skills per task based on the prompt
- Selection modes: `auto` (AI picks), `force` (user picks), `disable`
- New setting: `skillSelectorAgent` (default: "none")

### 3.4 Skill Injection

**What is needed:**
- Selected skills injected into generator and fixer prompts as context
- Modify `prompts/execution.rs` to accept optional skill instructions

### 3.5 Skill Enhancement

**What is needed:**
- AI refines skill descriptions/instructions when created
- Optional enhancement step before saving

### 3.6 Skill Marketplace

**What is needed:**
- Import skills from GitHub repositories
- Browse/search community skills

---

## Phase 4: Configuration & Polish — REMAINING

### 4.1 Per-Project Settings (LOW)

eaOrch stores settings per workspace in `.ea-orch/settings.json`. ea-code uses a global SQLite database.

**What is needed:**
- Project-scoped settings overrides table in DB
- Settings resolution: project override → global default
- UI to configure per-project settings

### 4.2 Missing Settings (LOW)

Settings present in eaOrch but missing in ea-code:

| Setting | eaOrch Default | Purpose |
|---------|---------------|---------|
| `agentTimeoutMs` | 0 (no timeout) | Per-agent timeout |
| `tokenOptimizedPrompts` | false | Use compact handoff mode |
| `copilotPath` | "gh" | GitHub Copilot CLI path |
| `mode` | "workspace-write" | Execution mode |
| `updateCliOnRun` | true | Auto-update CLIs on startup |
| `failOnCliUpdateError` | false | Stop if CLI update fails |
| `cliUpdateTimeoutMs` | 600000 | CLI update timeout |
| `agentMaxTurns` | 25 | Max agentic turns per CLI invocation |
| `planResponseTimeoutSec` | 45 | Auto-proceed timeout for plan gate |
| `maxPlanRevisions` | 3 | Max user revision rounds |
| `skillSelectorAgent` | "none" | Agent for skill selection |

### 4.3 Copilot Agent Adapter (LOW)

eaOrch supports GitHub Copilot via `gh copilot` CLI.

**What is needed:**
- New agent adapter in `src-tauri/src/agents/copilot.rs`
- Add `AgentBackend::Copilot` variant
- Update dispatch in `orchestrator/helpers.rs`
- Add `copilotPath` and `copilotModel` settings

### 4.4 MCP Server Catalog & Multi-CLI Configuration (LOW)

eaOrch has a pre-defined MCP server catalog and per-CLI MCP config. ea-code has basic Claude MCP support only.

**What is needed:**
- Pre-defined MCP server catalog (Context7, GitHub, Slack, etc.)
- Per-CLI MCP configuration
- MCP state persistence
- `getMcpCapableClis()` to filter which CLIs support MCP
- UI for MCP server management

---

## Architecture Comparison (Current State)

| Aspect | eaOrch (VS Code Extension) | ea-code (Tauri App) | Status |
|--------|---------------------------|---------------------|--------|
| Runtime | VS Code Extension Host | Tauri v2 (Rust + React) | Different by design |
| Storage | `.ea-orch/` JSON files | SQLite (Diesel ORM) | Improved |
| UI | Webview sidebar panel | Full React frontend | Improved |
| Agent count | 10 agents | 5 agents | Copilot missing |
| Prompt version | v2.5.0 (rich, structured) | v2.5.0 (ported) | **Done** |
| Pipeline stages | 10+ stages | 10 stages (full depth) | **Done** |
| Iteration loop | Full self-improvement with handoff JSON | Full handoff JSON | **Done** |
| Skills system | Full CRUD + marketplace + AI selection | Missing | Phase 3 |
| MCP integration | Catalog, per-CLI config, persistence | Partial (Claude only) | Phase 4 |
| Token optimisation | Dual mode (full diff vs git inspection) | Missing | Phase 2 |
| Plan user gate | Interactive approve/revise/skip with timeout | Missing | Phase 2 |
| Retry logic | 1 retry with prompt augmentation | Missing | Phase 2 |
| Per-project settings | Workspace-scoped | Global only | Phase 4 |

---

## Priority Summary

| Phase | Scope | Impact | Effort |
|-------|-------|--------|--------|
| **Phase 1** | Prompt engineering + module split | Highest | **COMPLETED** |
| **Phase 2** | Plan gate, retry, token optimisation | High | Medium |
| **Phase 3** | Skills system (data, CRUD, selection, injection, marketplace) | High | Large |
| **Phase 4** | Per-project settings, missing settings, Copilot, MCP catalog | Low-Medium | Medium |
