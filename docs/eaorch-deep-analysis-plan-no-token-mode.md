# EA Orchestrator Deep Analysis (Excluding Token-Optimised Mode)

## Scope

This report compares:

- Source extension: `C:\Github\eaOrch - Copy\ea-orchestrator`
- Target app: `C:\Github\ea-code`

Focus areas:

1. Orchestration flow
2. Prompt architecture
3. Settings and persistence
4. Missing features and implementation plan

Explicitly out of scope for this plan:

- Token-optimised prompt mode enhancements

---

## 1) Source Extension: Verified Structure and Capabilities

### 1.1 Main architecture

- VS Code extension host + webview sidebar.
- Core pipeline in `src/orchestrator/pipeline.ts`.
- Command entrypoints:
  - `src/commands/runPipeline.ts`
  - `src/commands/runDirectModel.ts`
- Agent registry and adapters:
  - `src/agents/registry.ts`
  - `src/agents/*.ts`

### 1.2 Orchestration stages in practice

Observed stage flow:

1. Prompt enhancement (optional via `none`)
2. Planning (optional)
3. Plan auditing (optional)
4. Plan gate (approve/revise/skip with timeout)
5. Skill selection (auto/force/disable)
6. Generation
7. Diff capture
8. Review
9. Fix
10. Diff capture
11. Judge
12. Iterative loop with handoff JSON
13. Optional summariser post-run

### 1.3 Prompt system

- Prompt version: `2.5.0`
- Prompt modules:
  - `prompts/enhancer.ts`
  - `prompts/planner.ts`
  - `prompts/planAuditor.ts`
  - `prompts/skillSelector.ts`
  - `prompts/generator.ts`
  - `prompts/reviewer.ts`
  - `prompts/fixer.ts`
  - `prompts/judge.ts`
  - `prompts/summariser.ts`
- Structured handoff parsing and fallback:
  - `orchestrator/judge.ts`

### 1.4 Settings model

Two-layer model:

- Global VS Code settings (`eaOrch.*`) in `package.json`.
- Workspace settings in `.ea-orch/settings.json` via `infra/eaOrchStore.ts`.

### 1.5 Extension-specific feature systems

- Skills system:
  - Storage in `.ea-orch/skills/<id>/SKILL.md`
  - Selector + enhancer prompts
  - Marketplace import from GitHub repos
- MCP system:
  - Catalog-backed UI state in `.ea-orch/mcp-servers.json`
  - Per-CLI configure/remove/list flows
- Startup CLI update checks:
  - `orchestrator/cliUpdates.ts`
  - Controlled by `updateCliOnRun`, `failOnCliUpdateError`, `cliUpdateTimeoutMs`

---

## 2) EA Code (Rust/Tauri): Verified Current State

### 2.1 Main architecture

- Tauri v2 app with Rust backend and React frontend.
- Pipeline engine in:
  - `src-tauri/src/orchestrator/pipeline.rs`
  - `src-tauri/src/orchestrator/iteration.rs`
  - `src-tauri/src/orchestrator/iteration_review.rs`

### 2.2 Existing orchestration capability

- Prompt enhancer stage
- Planner + plan auditor stages (conditionally enabled)
- Plan gate with timeout and revision loop
- Generate, review, fix, judge loop
- Retry with prompt augmentation (`agent_retry_count`)
- Judge verdict parsing with exact/checklist/heuristic fallback
- Iteration handoff JSON parsing and fallback
- Run-level executive summary stage

### 2.3 Prompt system

- Prompt version: `2.5.0`
- Prompt modules:
  - `prompts/planning.rs`
  - `prompts/execution.rs`
  - `prompts/judge.rs`
  - `prompts/mod.rs`

### 2.4 Settings and persistence

- Single global settings row in SQLite (`settings` table).
- Settings model in:
  - `src-tauri/src/models/settings.rs`
  - `src-tauri/src/db/models/settings.rs`
  - `src-tauri/src/db/settings.rs`
- No project-level override table.

### 2.5 Current feature boundaries

- Skills system: missing
- Skill selection stage: missing
- Copilot backend: missing
- MCP management/catalog UI: missing (only Claude MCP config generation exists)
- Rich workspace context summary builder: missing
- Startup CLI auto-update policy settings parity: missing

---

## 3) Deep Comparison (Flow, Prompts, Settings)

### 3.1 Orchestration flow parity

| Area | Extension | EA Code | Gap |
|---|---|---|---|
| Plan gate | Approve/revise/skip/timeout | Implemented with timeout + revisions | Near parity |
| Skills stage | Present before generation | Absent | Missing |
| Summariser | Optional post-run agent | Run-level executive summary stage | Behaviour differs |
| Copilot backend | Present | Absent | Missing |
| Per-run direct model mode | Present | Absent | Optional gap |

### 3.2 Prompt architecture parity

| Area | Extension | EA Code | Gap |
|---|---|---|---|
| Core prompts (enhance/plan/audit/generate/review/fix/judge) | Present | Present | Parity |
| Skill selector/enhancer prompts | Present | Absent | Missing |
| Summariser prompt | Present | Present | Parity with different usage |
| Handoff JSON handling | Present | Present | Parity |

### 3.3 Settings parity

| Setting/Capability | Extension | EA Code | Gap |
|---|---|---|---|
| Workspace-scoped settings | `.ea-orch/settings.json` | Global SQLite row | Missing |
| `skillSelectorAgent` | Yes | No | Missing |
| `summariserAgent` optionality | Yes (`none` allowed) | `executiveSummaryAgent` required | Behaviour differs |
| `agentMaxTurns` | Yes | No | Missing |
| `mode` (`workspace-write`/`diff-first`) | Yes | No | Missing |
| `updateCliOnRun` | Yes | No | Missing |
| `failOnCliUpdateError` | Yes | No | Missing |
| `cliUpdateTimeoutMs` | Yes | No | Missing |
| `copilotPath` | Yes | No | Missing |

---

## 4) Priority Gaps (Token-Optimised Work Excluded)

## P0

1. Skills data model + CRUD + UI
2. Skill selection stage in pipeline
3. Skill prompt injection into generator/fixer

## P1

1. Per-project settings overrides
2. Copilot backend adapter + settings + UI
3. MCP catalog and server management with persistent bindings

## P2

1. Rich workspace context summary (branch, package metadata, src tree, tests, capped length)
2. Startup CLI update policy parity (`updateCliOnRun`, `failOnCliUpdateError`, `cliUpdateTimeoutMs`)
3. `agentMaxTurns` and `mode` settings parity

---

## 5) Implementation Plan (Execution Order)

### Phase A: Skills Foundation (P0)

Deliver:

1. `skills` table + Diesel models + Rust commands
2. `useSkills` hook + Skills view components
3. Selector prompt + parser + pipeline skill selection stage
4. Selected skill injection into generator/fixer prompts

Acceptance:

1. User can create/edit/delete skills.
2. Auto mode selects up to 3 skills.
3. Selected skills are visible in run artefacts and affect prompts.

### Phase B: Settings Scope and Agent Parity (P1)

Deliver:

1. `project_settings` table and merge logic (global + project override)
2. Copilot backend in Rust agent dispatch + settings/UI exposure
3. Settings additions: `copilotPath`, `agentMaxTurns`, `mode`

Acceptance:

1. Two projects can run with different agent/stage settings.
2. Copilot can be selected and invoked in pipeline stages.
3. New settings persist and round-trip via frontend/backend.

### Phase C: MCP Parity (P1)

Deliver:

1. MCP server and binding tables
2. MCP CRUD commands + React management view
3. Adapter-level MCP config for supported CLIs
4. Built-in MCP catalogue seeding on migration

Acceptance:

1. Users can enable/disable MCP servers per CLI.
2. Configuration survives app restarts.
3. Agent runs load active MCP configuration correctly.

### Phase D: Operational and Context Improvements (P2)

Deliver:

1. Rich context summary builder in Rust orchestrator
2. CLI update policy settings and startup updater orchestration

Acceptance:

1. Context summary is included in relevant stages and length-capped.
2. Startup update policies behave per configuration and failure policy.

---

## 6) Build and Verification Gate

For implementation phases touching Rust and TypeScript:

1. `cd src-tauri && cargo check`
2. `npx tsc --noEmit`

No phase is complete until both pass for touched areas.

---

## 7) Recommended Start

Start with **Phase A (Skills Foundation)**, then **Phase B (Settings scope + Copilot)**.

Reason:

- Skills is the highest orchestration and prompt-quality multiplier.
- Project-scoped settings is required to avoid global coupling when skills and agent mixes grow.
