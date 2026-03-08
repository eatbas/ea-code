# EA Orchestrator → EA Code: Feature Gap Analysis

## Context

The original **eaOrch** was a VS Code extension (~13,800 LOC TypeScript) orchestrating multiple AI CLIs (Claude, Codex, Gemini, Kimi, Copilot, OpenCode) in a self-improving dev loop. It has been partially ported to **ea-code**, a Tauri v2 desktop application (Rust backend + React frontend, ~8,000 LOC).

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

---

## Phase 2: Pipeline Enhancements — COMPLETED

All items in Phase 2 have been implemented on `main`.

| Item | Status |
|------|--------|
| Plan User Gate — approve/revise/skip with auto-approve timeout | Done |
| Retry with Prompt Augmentation — 1+ retries with `PREVIOUS ATTEMPT FAILED` hint | Done |
| New settings: `requirePlanApproval`, `planAutoApproveTimeoutSec`, `maxPlanRevisions` | Done |
| New settings: `agentRetryCount`, `agentTimeoutMs`, `tokenOptimizedPrompts` | Done |
| DB migration for Phase 2 columns (`settings` + `iterations` tables) | Done |
| Frontend types updated (`AppSettings` interface) | Done |
| Settings UI updated (Pipeline section + Plan Gate section) | Done |
| Fix pre-existing `generate_handler!` macro bug (use full submodule paths) | Done |
| Diesel `64-column-tables` feature for settings table (36 columns) | Done |
| File splits to stay under 300-line limit (`helpers.rs` → `user_questions.rs`, `runs.rs` → `run_detail.rs`) | Done |

### Phase 2 New Files

| File | Lines | Purpose |
|------|-------|---------|
| `src-tauri/src/orchestrator/plan_gate.rs` | 141 | Plan user gate: approve/revise/skip with timeout |
| `src-tauri/src/orchestrator/user_questions.rs` | 156 | Ask-user-question helpers (extracted from helpers.rs) |
| `src-tauri/src/db/run_detail.rs` | 118 | Run listing and full detail queries (extracted from runs.rs) |
| `src-tauri/migrations/2026-03-08-000008_add_phase2_settings/` | — | DB migration for 6 settings + 2 iteration columns |

### Phase 2 Implementation Details

#### Plan User Gate
- Pipeline pauses after planning if `requirePlanApproval` is enabled and a plan exists
- User can: **approve**, **request revisions** (up to `maxPlanRevisions`), or **skip**
- Auto-approve timeout: configurable via `planAutoApproveTimeoutSec` (default 45s, 0 = wait indefinitely)
- Revision loop: user feedback → planner re-plans → user re-reviews
- Approval statuses tracked per iteration: `approved`, `skipped`, `approved_max_revisions`, `approved_revision_failed`

#### Retry with Prompt Augmentation
- Configurable retry count per agent call via `agentRetryCount` (default 1)
- On retry, augments prompt with `PREVIOUS ATTEMPT FAILED (attempt N of M): {error_message}`
- Cancellation and abort errors are never retried
- Each retry is logged as a separate stage record with the attempt number

### Note on Token-Optimised Prompts
The `tokenOptimizedPrompts` setting column exists in the database but the **prompt-level implementation has been deferred**. The setting is wired through the full stack (DB → Rust → Frontend) but has no effect on prompt generation yet. See Phase 5 for full implementation plan.

---

## Phase 3: Skills System — REMAINING (Entirely Missing)

eaOrch has a complete skills system; ea-code has nothing. This is a high-impact feature that allows users to teach the pipeline domain-specific knowledge (coding conventions, API patterns, deployment procedures) that persist across sessions.

### eaOrch Skills Architecture (Reference)

In eaOrch, skills are stored as JSON files under `.ea-orch/skills/`:
```json
{
  "id": "uuid",
  "name": "React Component Standards",
  "description": "Conventions for building React components in this project",
  "instructions": "- Use functional components only\n- Props must have explicit interfaces\n- Use named exports\n- Co-locate tests with components",
  "tags": ["react", "frontend", "components"],
  "createdAt": "2026-01-15T10:00:00Z",
  "updatedAt": "2026-02-20T14:30:00Z"
}
```

The skills flow in eaOrch:
1. User prompt arrives
2. Skill selector agent reads all skills, picks 0-3 most relevant
3. Selected skill instructions are injected into the generator and fixer system prompts
4. Pipeline runs with domain-specific context

Selection modes: `auto` (AI picks), `force` (user selects from list), `disable` (no skills).

---

### 3.1 Skill Data Model & Migration

**DB table schema:**

```sql
CREATE TABLE skills (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    instructions TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '',           -- Comma-separated
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Rust model:**

```rust
#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = skills)]
#[serde(rename_all = "camelCase")]
pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tags: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}
```

**Frontend type:**

```typescript
export interface Skill {
  id: string;
  name: string;
  description: string;
  instructions: string;
  tags: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}
```

**Files to create:**
| File | Purpose |
|------|---------|
| `src-tauri/migrations/NNNN_create_skills/up.sql` | Create `skills` table |
| `src-tauri/migrations/NNNN_create_skills/down.sql` | Drop `skills` table |
| `src-tauri/src/db/skills.rs` | Diesel CRUD queries |
| `src-tauri/src/db/models/skills.rs` | `SkillRow`, `NewSkill` structs |
| `src-tauri/src/models/skills.rs` | Frontend-facing `Skill` type |
| `src/types/skills.ts` | TypeScript `Skill` interface |

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/src/schema.rs` | Add `skills` table definition |
| `src-tauri/src/db/mod.rs` | Add `pub mod skills;` |
| `src-tauri/src/db/models/mod.rs` | Add `pub mod skills;` and re-exports |
| `src-tauri/src/models/mod.rs` | Add `pub mod skills;` and re-exports |
| `src/types/index.ts` | Re-export skills types |

---

### 3.2 Skill CRUD Commands

**Tauri commands:**

| Command | Signature | Purpose |
|---------|-----------|---------|
| `list_skills` | `() → Vec<Skill>` | List all skills |
| `get_skill` | `(id: String) → Skill` | Get a single skill by ID |
| `create_skill` | `(name, description, instructions, tags) → Skill` | Create a new skill |
| `update_skill` | `(id, name?, description?, instructions?, tags?, isActive?) → Skill` | Update a skill |
| `delete_skill` | `(id: String) → ()` | Delete a skill |
| `import_skill` | `(json: String) → Skill` | Import a skill from JSON |
| `export_skill` | `(id: String) → String` | Export a skill as JSON |

**Files to create:**
| File | Purpose |
|------|---------|
| `src-tauri/src/commands/skills.rs` | Tauri command handlers |
| `src/hooks/useSkills.ts` | React hook: `{ skills, loading, createSkill, updateSkill, deleteSkill }` |

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/src/commands/mod.rs` | Add `pub(crate) mod skills;` |
| `src-tauri/src/lib.rs` | Register skill commands in `generate_handler!` |

---

### 3.3 Skill Management UI

**Components to create:**

| Component | Purpose |
|-----------|---------|
| `src/components/SkillsView/index.tsx` | Main skills management page |
| `src/components/SkillsView/SkillCard.tsx` | Individual skill card with edit/delete |
| `src/components/SkillsView/SkillEditor.tsx` | Modal for creating/editing a skill |

**UI design:**
- Grid/list of skill cards showing name, description, tags, active status
- "Add Skill" button opens editor modal
- Each card has edit, toggle active, delete actions
- Tags rendered as coloured chips
- Instructions shown in a monospaced text area
- Import/export buttons for sharing skills

**Files to modify:**
| File | Change |
|------|--------|
| `src/components/Sidebar.tsx` | Add "Skills" navigation item |
| `src/App.tsx` | Add skills route/view |

---

### 3.4 Skill Selection (AI-Powered)

When `skillSelectorAgent` is set to a backend (not "none"), the pipeline runs a skill selection step before generation:

**Selection prompt (for the selector agent):**
```
You are a skill selector. Given the user's task and a list of available skills,
select 0-3 skills that are most relevant.

Reply with a JSON array of skill IDs, e.g. ["id1", "id2"].
If no skills are relevant, reply with [].

Available skills:
{skills_json}

User task:
{enhanced_prompt}
```

**Implementation:**

| File | Change |
|------|--------|
| `src-tauri/src/orchestrator/iteration.rs` | Add skill selection step before generation |
| `src-tauri/src/orchestrator/prompts/execution.rs` | Accept `selected_skills: &[Skill]` parameter |
| `src-tauri/src/orchestrator/run_setup.rs` | Add `selected_skills: Vec<Skill>` to `IterationContext` |
| `src-tauri/src/models/settings.rs` | Add `skill_selector_agent: Option<AgentBackend>` and `skill_selection_mode: String` |

**Selection modes:**

| Mode | Behaviour |
|------|-----------|
| `disable` | No skill selection; skills ignored entirely |
| `auto` | AI agent selects 0-3 skills based on the prompt (default when agent is set) |
| `force` | Pipeline pauses, user selects skills from a list before generation |

**New settings:**

| Setting | Type | Default | Purpose |
|---------|------|---------|---------|
| `skillSelectorAgent` | `Option<AgentBackend>` | `None` | Which agent runs skill selection |
| `skillSelectionMode` | `String` | `"disable"` | `disable`, `auto`, or `force` |

---

### 3.5 Skill Injection into Prompts

Selected skills are injected into the **generator** and **fixer** system prompts as an additional context section:

```
--- RELEVANT SKILLS ---
The following domain-specific skills have been selected for this task.
Follow these instructions in addition to the standard guidelines.

### Skill: {skill.name}
{skill.instructions}

### Skill: {skill.name}
{skill.instructions}
--- END SKILLS ---
```

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/src/orchestrator/prompts/execution.rs` | `build_generator_system()` and `build_fixer_system()` accept optional `&[Skill]` |
| `src-tauri/src/orchestrator/iteration.rs` | Pass selected skills to prompt builders |

---

### 3.6 Skill Enhancement (Optional)

When creating a skill, the user can optionally run an AI agent to refine the skill's description and instructions:

**Enhancement prompt:**
```
Improve this skill definition for clarity and completeness.
Ensure the instructions are specific, actionable, and well-structured.

Name: {name}
Description: {description}
Instructions: {instructions}

Reply with improved JSON: {"name": "...", "description": "...", "instructions": "..."}
```

**Implementation:**
- Add "Enhance with AI" button in `SkillEditor.tsx`
- Tauri command `enhance_skill(name, description, instructions) → EnhancedSkill`
- Uses the `promptEnhancerAgent` backend

---

### 3.7 Skill Marketplace (Future)

Import skills from GitHub repositories. This is lower priority and can be a stretch goal.

**Concept:**
- Skills stored as JSON files in a GitHub repo (e.g. `ea-code-skills/`)
- "Browse Marketplace" button fetches repo contents via GitHub API
- User previews and imports individual skills
- Imported skills are saved to the local DB

**Files to create (when implemented):**
| File | Purpose |
|------|---------|
| `src-tauri/src/skills/marketplace.rs` | GitHub API client for skill repos |
| `src/components/SkillsView/Marketplace.tsx` | Browse/import UI |

---

### Phase 3 Effort Estimate

| Sub-task | Effort | Priority |
|----------|--------|----------|
| 3.1 Data model & migration | Small | Required |
| 3.2 CRUD commands | Small | Required |
| 3.3 Management UI | Medium | Required |
| 3.4 AI-powered selection | Medium | High |
| 3.5 Prompt injection | Small | High |
| 3.6 Skill enhancement | Small | Medium |
| 3.7 Marketplace | Medium | Low (stretch) |

---

## Phase 4: Configuration & Polish — REMAINING

### 4.1 Per-Project Settings (MEDIUM)

ea-code currently uses a single global settings row. eaOrch stores settings per workspace in `.ea-orch/settings.json`, allowing different agent configurations per project.

**Approach:** Project-scoped settings overrides stored in a new `project_settings` table. Each row overrides specific fields; unset fields fall back to the global default.

**DB schema:**
```sql
CREATE TABLE project_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    setting_key TEXT NOT NULL,       -- e.g. "generatorAgent", "maxIterations"
    setting_value TEXT NOT NULL,     -- JSON-encoded value
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, setting_key)
);
```

**Resolution logic:**
1. Load global `AppSettings` from the `settings` table
2. Load project overrides from `project_settings` for the active project
3. Merge: override values replace globals, unset keys use global defaults
4. Pass merged settings to the pipeline

**Files to create:**
| File | Purpose |
|------|---------|
| `src-tauri/migrations/NNNN_project_settings/up.sql` | Create `project_settings` table |
| `src-tauri/src/db/project_settings.rs` | CRUD for per-project overrides |
| `src/components/ProjectSettingsView.tsx` | UI for per-project setting overrides |

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/src/schema.rs` | Add `project_settings` table |
| `src-tauri/src/db/mod.rs` | Add `pub mod project_settings;` |
| `src-tauri/src/db/settings.rs` | Add `get_merged(pool, project_id) → AppSettings` function |
| `src-tauri/src/orchestrator/pipeline.rs` | Load merged settings using active project ID |
| `src/components/Sidebar.tsx` | "Project Settings" link when a project is active |

---

### 4.2 Missing Settings (LOW)

Settings present in eaOrch but not yet in ea-code. These should be added to `AppSettings`, the DB migration, and the settings UI as needed.

| Setting | Type | eaOrch Default | Purpose | Priority |
|---------|------|---------------|---------|----------|
| `agentMaxTurns` | `u32` | 25 | Max agentic turns per CLI invocation | Medium |
| `mode` | `String` | "workspace-write" | Execution mode (`workspace-write` vs `diff-first`) | Low |
| `updateCliOnRun` | `bool` | true | Auto-update CLIs before each run | Low |
| `failOnCliUpdateError` | `bool` | false | Abort run if CLI update fails | Low |
| `cliUpdateTimeoutMs` | `u64` | 600000 | CLI update timeout | Low |

**Implementation pattern (per setting):**
1. Add column to `settings` table via new migration
2. Add field to `SettingsRow`, `SettingsChangeset` (Rust DB models)
3. Add field to `AppSettings` (Rust application model) with default
4. Update `row_to_app_settings()` and the changeset builder in `db/settings.rs`
5. Add field to `AppSettings` interface (TypeScript)
6. Add UI control in `SettingsView.tsx` and `SettingsPanel.tsx`
7. Update `schema.rs` (auto-generated or manual)

**Note:** `agentMaxTurns` is the most impactful of these — it controls how many tool-use rounds an agent CLI gets. This is already supported by Claude CLI (`--max-turns`) but ea-code doesn't pass it. The others are operational settings.

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/migrations/NNNN_add_phase4_settings/up.sql` | New columns |
| `src-tauri/src/schema.rs` | New column definitions |
| `src-tauri/src/db/models/settings.rs` | New fields on Row + Changeset |
| `src-tauri/src/models/settings.rs` | New fields on AppSettings |
| `src-tauri/src/db/settings.rs` | Changeset + conversion updates |
| `src/types/settings.ts` | New fields + defaults |
| `src/components/SettingsView.tsx` | New UI controls |
| `src/components/SettingsPanel.tsx` | New UI controls |

---

### 4.3 Copilot Agent Adapter (LOW)

eaOrch supports GitHub Copilot via `gh copilot suggest` / `gh copilot explain` CLI commands. ea-code has 5 agents (Claude, Codex, Gemini, Kimi, OpenCode) — Copilot is missing.

**eaOrch Copilot integration:**
- Path setting: `copilotPath` (default `"gh"`)
- Invocation: `gh copilot suggest -t code "{prompt}"`
- No MCP support
- No session/conversation continuity (stateless per call)

**Implementation:**

| File | Purpose |
|------|---------|
| `src-tauri/src/agents/copilot.rs` | New agent adapter (~30 lines, following codex.rs pattern) |

| File | Change |
|------|--------|
| `src-tauri/src/agents/mod.rs` | Add `pub mod copilot;` and re-export `run_copilot` |
| `src-tauri/src/models/agents.rs` | Add `Copilot` variant to `AgentBackend` enum |
| `src-tauri/src/orchestrator/helpers.rs` | Add `AgentBackend::Copilot` arm in `dispatch_agent()` |
| `src-tauri/src/models/settings.rs` | Add `copilot_path: String` and `copilot_model: String` |
| `src-tauri/src/db/models/settings.rs` | Add columns to Row + Changeset |
| `src/types/agents.ts` | Add `"copilot"` to `AgentBackend` type |
| `src/types/settings.ts` | Add `copilotPath` and `copilotModel` fields |
| `src/components/shared/FormInputs.tsx` | Add Copilot to agent select options |
| `src/components/CliSetupView.tsx` | Add Copilot path input + health check |

**Agent adapter pattern** (follows existing thin wrappers):
```rust
pub async fn run_copilot(
    input: &AgentInput,
    cli_path: &str,
    model: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    run_cli_agent(
        input, cli_path,
        &["copilot", "suggest", "-t", "code"],
        model, app, run_id, stage, db,
    ).await
}
```

---

### 4.4 MCP Server Catalog & Multi-CLI Configuration (MEDIUM)

ea-code currently has MCP support for **Claude only** (`agents/claude.rs` generates a temporary MCP config). eaOrch has a richer MCP system:

**eaOrch MCP features:**
- Pre-defined MCP server catalog with install instructions
- Per-CLI MCP configuration (which servers each CLI gets)
- MCP state persistence (`.ea-orch/mcp-servers.json`)
- `getMcpCapableClis()` — not all CLIs support MCP
- UI to enable/disable individual MCP servers

**eaOrch MCP server catalog (built-in):**

| Server | Purpose | Install |
|--------|---------|---------|
| Context7 | Library documentation lookup | `npx -y @context7/mcp` |
| GitHub | GitHub API access | `npx -y @modelcontextprotocol/server-github` |
| Slack | Slack workspace access | `npx -y @anthropic/slack-mcp` |
| Filesystem | Local file access | `npx -y @modelcontextprotocol/server-filesystem` |
| Brave Search | Web search | `npx -y @anthropic/brave-search-mcp` |

**MCP-capable CLIs:** Claude, Codex (partial). Gemini, Kimi, OpenCode do not support MCP.

**Implementation plan:**

**DB schema for MCP servers:**
```sql
CREATE TABLE mcp_servers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    command TEXT NOT NULL,             -- e.g. "npx"
    args TEXT NOT NULL DEFAULT '[]',   -- JSON array of args
    env TEXT NOT NULL DEFAULT '{}',    -- JSON object of env vars
    is_enabled BOOLEAN NOT NULL DEFAULT 0,
    is_builtin BOOLEAN NOT NULL DEFAULT 0,  -- From catalog vs user-defined
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Which CLIs get which MCP servers
CREATE TABLE cli_mcp_bindings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cli_name TEXT NOT NULL,            -- "claude", "codex"
    mcp_server_id TEXT NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE,
    UNIQUE(cli_name, mcp_server_id)
);
```

**Files to create:**
| File | Purpose |
|------|---------|
| `src-tauri/migrations/NNNN_mcp_servers/up.sql` | Create MCP tables |
| `src-tauri/src/db/mcp.rs` | MCP server CRUD + binding queries |
| `src-tauri/src/db/models/mcp.rs` | `McpServerRow`, `NewMcpServer`, `CliMcpBinding` |
| `src-tauri/src/models/mcp.rs` | Frontend-facing `McpServer` type |
| `src-tauri/src/commands/mcp.rs` | Tauri commands for MCP management |
| `src/types/mcp.ts` | TypeScript `McpServer` interface |
| `src/hooks/useMcpServers.ts` | React hook for MCP CRUD |
| `src/components/McpView/index.tsx` | MCP server management page |
| `src/components/McpView/ServerCard.tsx` | Individual server card UI |
| `src/components/McpView/AddServerDialog.tsx` | Add custom MCP server dialog |

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/src/agents/claude.rs` | Use `db/mcp.rs` to load enabled servers instead of hardcoded config |
| `src-tauri/src/agents/codex.rs` | Add MCP config support (if Codex supports it) |
| `src-tauri/src/schema.rs` | Add `mcp_servers` + `cli_mcp_bindings` tables |
| `src-tauri/src/db/mod.rs` | Add `pub mod mcp;` |
| `src-tauri/src/commands/mod.rs` | Add `pub(crate) mod mcp;` |
| `src-tauri/src/lib.rs` | Register MCP commands |
| `src/components/Sidebar.tsx` | Add "MCP Servers" navigation item |
| `src/App.tsx` | Add MCP route/view |

**Startup seeding:** On first launch (or after migration), seed the `mcp_servers` table with the built-in catalog entries (`is_builtin = true`, `is_enabled = false`). Users can then enable the ones they want.

---

### 4.5 Token-Optimised Prompt Mode (DEFERRED)

The `tokenOptimizedPrompts` setting exists in the database and UI but has no prompt-level effect. When implemented:

**eaOrch behaviour when `tokenOptimizedPrompts = true`:**

| Change | Standard Mode | Token-Optimised Mode |
|--------|--------------|---------------------|
| Reviewer input | Full git diff embedded in prompt | Reviewer told to run `git diff HEAD~1` locally |
| Judge input | Full git diff embedded | Judge told to run `git diff HEAD~1` locally |
| Inter-iteration context | Full judge output (~2000+ tokens) | Structured `IterationHandoff` JSON (~200 tokens) |
| Diff stage | Captures diff, embeds in next prompt | Captures diff, stores but doesn't embed |

**Files to modify (when implemented):**
| File | Change |
|------|--------|
| `src-tauri/src/orchestrator/prompts/execution.rs` | Conditional reviewer prompt (git inspection vs inline diff) |
| `src-tauri/src/orchestrator/prompts/judge.rs` | Conditional judge prompt (git inspection vs inline diff) |
| `src-tauri/src/orchestrator/iteration.rs` | Skip diff embedding when token-optimised |
| `src-tauri/src/orchestrator/iteration_review.rs` | Skip diff embedding in review/judge stages |

**Impact:** Reduces token usage by 40-60% on large codebases by not embedding full diffs inline. Trade-off is that agents must have `git` available and the workspace must be a git repo.

---

### 4.6 Richer Context Summary Builder (LOW)

eaOrch builds a context snapshot of the workspace before each run:

| Context Element | eaOrch | ea-code | Gap |
|----------------|--------|---------|-----|
| Git branch | Yes | Yes (`git::workspace_info()`) | Parity |
| `package.json` metadata | Scripts, deps | No | Missing |
| Open editor contents | Up to 5 files | N/A (not a VS Code extension) | By design |
| `src/` directory tree | 2 levels deep | No | Missing |
| README.md head | First 60 lines | No | Missing |
| Discovered test files | Yes | No | Missing |
| Hard character cap | 8000 chars | No cap | Missing |

**Note:** Since ea-code is a standalone desktop app (not a VS Code extension), it cannot access open editor contents. However, the other context elements are achievable.

**Files to modify:**
| File | Change |
|------|--------|
| `src-tauri/src/orchestrator/pipeline.rs` | Build richer context snapshot at run start |
| `src-tauri/src/orchestrator/run_setup.rs` | Add context summary fields to run state |

**Implementation:**
```rust
pub fn build_context_summary(workspace: &str) -> String {
    let mut parts = Vec::new();
    // 1. Git branch
    // 2. package.json scripts/deps (if exists)
    // 3. src/ tree (2 levels)
    // 4. README.md head (60 lines)
    // 5. Test file discovery (*.test.*, *.spec.*)
    // Truncate to 8000 chars
    parts.join("\n\n")
}
```

---

### Phase 4 Effort Estimate

| Sub-task | Effort | Priority |
|----------|--------|----------|
| 4.1 Per-project settings | Medium | Medium |
| 4.2 Missing settings | Small | Low |
| 4.3 Copilot adapter | Small | Low |
| 4.4 MCP catalog | Large | Medium |
| 4.5 Token-optimised prompts | Medium | Medium (deferred) |
| 4.6 Context summary builder | Small | Low |

---

## Architecture Comparison (Current State)

| Aspect | eaOrch (VS Code Extension) | ea-code (Tauri App) | Status |
|--------|---------------------------|---------------------|--------|
| Runtime | VS Code Extension Host | Tauri v2 (Rust + React) | Different by design |
| Storage | `.ea-orch/` JSON files | SQLite (Diesel ORM) | Improved |
| UI | Webview sidebar panel | Full React frontend | Improved |
| Agent count | 10 agents | 5 agents | Copilot missing (Phase 4) |
| Prompt version | v2.5.0 (rich, structured) | v2.5.0 (ported) | **Done** |
| Pipeline stages | 10+ stages | 10 stages (full depth) | **Done** |
| Iteration loop | Full self-improvement with handoff JSON | Full handoff JSON | **Done** |
| Plan user gate | Interactive approve/revise/skip with timeout | Full implementation | **Done** |
| Retry logic | 1 retry with prompt augmentation | Full implementation | **Done** |
| Skills system | Full CRUD + marketplace + AI selection | Missing | Phase 3 |
| MCP integration | Catalog, per-CLI config, persistence | Partial (Claude only) | Phase 4 |
| Token optimisation | Dual mode (full diff vs git inspection) | Setting exists, prompts deferred | Phase 4 |
| Per-project settings | Workspace-scoped | Global only | Phase 4 |
| Context summary | Rich (git, deps, tree, README, tests) | Basic (git only) | Phase 4 |

---

## Current Codebase Structure

```
src-tauri/src/                          Lines
├── agents/
│   ├── mod.rs                           13    Module re-exports
│   ├── base.rs                         147    AgentInput/Output, build_full_prompt, run_cli_agent
│   ├── claude.rs                       128    Claude adapter with MCP config
│   ├── codex.rs                         29    Codex adapter (thin wrapper)
│   ├── gemini.rs                        29    Gemini adapter (thin wrapper)
│   ├── kimi.rs                          29    Kimi adapter (thin wrapper)
│   └── opencode.rs                      29    OpenCode adapter (thin wrapper)
├── commands/
│   ├── mod.rs                           21    AppState + submodule declarations
│   ├── cli.rs                                 CLI health/version commands
│   ├── history.rs                             Session/run/log/artifact queries
│   ├── pipeline.rs                            run_pipeline, cancel, answer_question
│   ├── settings.rs                            get_settings, save_settings
│   └── workspace.rs                           select_workspace, validate_environment
├── db/
│   ├── mod.rs                          102    Pool init, migrations, legacy import
│   ├── models/
│   │   ├── mod.rs                       12    Re-exports
│   │   ├── records.rs                  250    RunRow, IterationRow, StageRow
│   │   ├── settings.rs                  89    SettingsRow, SettingsChangeset (36 cols)
│   │   └── details.rs                   63    Query result structs
│   ├── artifacts.rs                     40    Artifact CRUD
│   ├── logs.rs                          47    Log line insertion
│   ├── projects.rs                      81    Project upsert
│   ├── questions.rs                     57    User question CRUD
│   ├── run_detail.rs                   118    Run listing + full detail queries
│   ├── runs.rs                         216    Run/iteration/stage CRUD + patches
│   ├── sessions.rs                     144    Session CRUD
│   └── settings.rs                     145    Settings get/update
├── orchestrator/
│   ├── mod.rs                           22    Module declarations
│   ├── pipeline.rs                     124    Main run_pipeline loop
│   ├── iteration.rs                    251    Single iteration + planning stages
│   ├── iteration_review.rs             180    Review/fix/judge sub-stages
│   ├── stages.rs                       210    execute_agent_stage with retry
│   ├── helpers.rs                      202    Dispatch, events, cancellation
│   ├── user_questions.rs               156    Ask-user-question helpers
│   ├── plan_gate.rs                    141    Plan approval gate
│   ├── run_setup.rs                    207    IterationContext, exec summary, teardown
│   ├── parsing.rs                      191    3-tier verdict parsing
│   └── prompts/
│       ├── mod.rs                      203    PromptMeta, IterationHandoff
│       ├── planning.rs                 177    Enhancer, planner, auditor prompts
│       ├── execution.rs                240    Generator, reviewer, fixer prompts
│       └── judge.rs                    196    Judge prompt with rubric
├── models/
│   ├── mod.rs                           11    Re-exports
│   ├── settings.rs                     129    AppSettings (30+ fields)
│   ├── pipeline.rs                      97    PipelineStage, Status, Request, Run
│   ├── environment.rs                   61    CLI paths, versions
│   ├── agents.rs                        54    AgentRole, AgentBackend enums
│   └── questions.rs                     38    PipelineQuestion/Answer
├── bin/mcp_server/                     426    Standalone MCP server binary
├── events.rs                                  Tauri event payload types
├── git.rs                                     Git workspace helpers
├── schema.rs                                  Diesel table definitions
└── lib.rs                               62    Tauri app builder

src/                                    Lines
├── types/
│   ├── index.ts                          5    Re-exports
│   ├── settings.ts                     114    AppSettings interface
│   ├── history.ts                      149    Run/Iteration/Stage/Session types
│   ├── pipeline.ts                      60    Pipeline request/response types
│   ├── events.ts                        67    Event payload types
│   └── agents.ts                        50    AgentRole/Backend enums
├── hooks/
│   ├── usePipeline.ts                  231    Pipeline execution hook
│   ├── useSettings.ts                   57    Settings load/save hook
│   ├── useHistory.ts                    56    History query hook
│   ├── useCliVersions.ts               54    CLI version hook
│   ├── useWorkspace.ts                  36    Workspace detection hook
│   └── useCliHealth.ts                  29    CLI health check hook
├── components/
│   ├── CliSetupView.tsx                291    CLI configuration view
│   ├── SettingsView.tsx                203    Global settings view
│   ├── SettingsPanel.tsx               219    Settings modal panel
│   ├── Sidebar.tsx                     199    Navigation sidebar
│   ├── AgentsView/index.tsx            127    Agent backend selection
│   ├── AgentsView/CascadingSelect.tsx  195    Agent/model cascading dropdown
│   ├── ChatView.tsx                    140    Pipeline chat interface
│   ├── RunTimeline.tsx                 182    Iteration/stage timeline
│   ├── IdleView.tsx                    127    Welcome screen
│   ├── StatusBar.tsx                   117    Status display
│   ├── QuestionDialog.tsx              106    User question modal
│   ├── shared/FormInputs.tsx            98    Reusable form components
│   ├── ArtifactsPanel.tsx               59    Artifact viewer
│   ├── Header.tsx                       47    App header
│   ├── LogsPanel.tsx                    41    Log viewer
│   └── shared/constants.ts             24    Form constants
└── App.tsx / main.tsx                         Root layout + entry point
```

**Total:** ~4,750 LOC Rust backend, ~3,270 LOC TypeScript frontend.

---

## Priority Summary

| Phase | Scope | Impact | Effort | Status |
|-------|-------|--------|--------|--------|
| **Phase 1** | Prompt engineering + module split | Highest | Medium | **COMPLETED** |
| **Phase 2** | Plan gate, retry, settings | High | Medium | **COMPLETED** |
| **Phase 3** | Skills system (data, CRUD, selection, injection, marketplace) | High | Large | Remaining |
| **Phase 4** | Per-project settings, Copilot, MCP catalog, token optimisation, context builder | Medium | Large | Remaining |

---

## Verification Plan

1. **Prompt quality**: Run the same task through both eaOrch and ea-code, compare agent outputs
2. **Judge reliability**: Verify structured rubric produces consistent COMPLETE/NOT COMPLETE verdicts
3. **Iteration handoff**: Run multi-iteration tasks, verify handoff JSON is correctly parsed and injected
4. **Plan gate**: Test approve, revise, skip, and timeout scenarios
5. **Skills**: Create skills, verify AI selection, confirm injection into prompts
6. **Token optimisation**: Compare token usage between standard and optimised modes
7. **MCP**: Verify multi-server configuration works across CLIs
8. **Build checks**: `cargo check` (Rust) and `npx tsc --noEmit` (TypeScript) after every change
