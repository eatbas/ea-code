# AGENTS.md - EA Code Agent Instructions

This guidance applies to any AI agent editing this repository.

## Project Snapshot

- The primary product is the desktop app in `frontend/desktop/`.
- Desktop stack: Tauri v2, React 19, TypeScript 5.8, Tailwind CSS v4, Rust backend.
- The repo also contains the marketing site in `frontend/web/` and release scripts in `scripts/`.
- Current persistence is file-based under `~/.ea-code/`. Older SQLite and `~/.config/ea-code/` references are legacy migration context, not the live architecture.

## Working Scope

- Default to `frontend/desktop/` unless the user explicitly asks for website work.
- Inspect `frontend/web/` only when the request says `website`, `web`, or points to that path.
- Ignore generated and dependency output unless the task requires it:
  - `frontend/desktop/node_modules/`
  - `frontend/desktop/src-tauri/target/`
  - `frontend/desktop/src-tauri/gen/`

## Verify Before Delivering

After touching desktop TypeScript or React:

```sh
cd frontend/desktop && npx tsc --noEmit
```

After touching desktop Rust or Tauri files:

```sh
cd frontend/desktop/src-tauri && cargo check
```

After touching website TypeScript or React:

```sh
cd frontend/web && npx tsc --noEmit
```

- If you only change docs or Markdown, no build check is required.
- Do not present code that fails the relevant verification step.

## File Size and Structure

- Keep new or substantially edited source files under 300 lines where practical.
- If a touched file is already over the limit, split it when the edit is non-trivial.
- Current over-limit source files are technical debt, not a pattern to copy.
- Prefer directory modules when splitting:
  - Rust: `feature/mod.rs` plus focused sibling modules
  - React: `Feature/index.tsx` plus subcomponents or helpers

## Reuse Before Adding

- Search for an existing pattern before creating a new file or abstraction.
- Shared frontend UI belongs in `frontend/desktop/src/components/shared/`.
- Shared frontend state logic belongs in `frontend/desktop/src/hooks/`.
- Frontend types are split by domain in `frontend/desktop/src/types/` and re-exported from `index.ts`.
- Frontend utilities live in `frontend/desktop/src/utils/`.
- Rust agent execution helpers live in `frontend/desktop/src-tauri/src/agents/base/`.
- Rust models are split by domain in `frontend/desktop/src-tauri/src/models/`.
- File persistence helpers live in `frontend/desktop/src-tauri/src/storage/`.

## Current Architecture

### Desktop Frontend

- Entry points: `frontend/desktop/src/main.tsx` (wraps App in StrictMode + ToastProvider) and `frontend/desktop/src/App.tsx` (initialises all hooks, renders layout).
- Global styles: `frontend/desktop/src/index.css` (CSS variables for dark theme, Tailwind v4 import, custom scrollbar, animations).
- View routing: `AppContentRouter.tsx` switches on an `ActiveView` enum: `"home"` | `"agents"` | `"cli-setup"` | `"skills"` | `"mcp"`.

#### Components (`src/components/`)

Top-level view components:

| Component | Purpose |
|-----------|---------|
| `AppContentRouter.tsx` | Routes between views based on `activeView` state |
| `ChatView.tsx` | Live pipeline execution UI with stage cards, logs, artefacts |
| `IdleView.tsx` | Landing screen with workspace selector and prompt input |
| `SessionDetailView.tsx` | Session detail with run history and pagination |
| `Sidebar.tsx` | Collapsible navigation sidebar with project/session list |
| `Header.tsx` | Top bar with workspace selector and settings button |
| `LogsPanel.tsx` | Pipeline log / terminal output display |
| `ProjectThreadsList.tsx` | Project and session list in sidebar |
| `QuestionDialog.tsx` | Dialog for answering pipeline questions mid-run |
| `RunCard.tsx` | Card for a completed/running pipeline run |
| `RunTimeline.tsx` | Visual timeline of pipeline iterations and stages |
| `SkillsView.tsx` | Skills catalogue CRUD interface |
| `StatusBar.tsx` | Pipeline status indicator bar |

Feature folders:

- `AgentsView/` — Agent backend and model configuration (`index.tsx`, `StageCard.tsx`, `CascadingSelect.tsx`, `InlineStageSlot.tsx`, `agentHelpers.ts`)
- `CliSetupView/` — CLI tool version checking and updates (`index.tsx`, `CliCard.tsx`)
- `McpView/` — MCP server management (`index.tsx`, `McpServerCard.tsx`, `helpers.ts`)
- `shared/` — Reusable UI: `ArtifactCard`, `AssistantMessageBubble`, `FinalPlanCard`, `FormInputs`, `PipelineControlBar`, `PopoverSelect`, `ProjectLoadingOverlay`, `PromptCard`, `PromptInputBar`, `PromptReceivedCard`, `RecentTerminalPanel`, `ResultCard`, `RichStageCard`, `StageCard`, `StageInputOutputCard`, `ThinkingIndicator`, `Toast`, `UpdateInstallBanner`, `WorkspaceFooter`, `constants.ts`

#### Hooks (`src/hooks/`)

| Hook | Purpose |
|------|---------|
| `useAppViewState` | Owns view state, session selection, navigation, live session polling |
| `useClickOutside` | Close dropdown/popover on outside click or Escape |
| `useCliHealth` | Check CLI backend availability (event-driven) |
| `useCliVersions` | Fetch CLI versions and trigger updates (event-driven) |
| `useElapsedTimer` | Format elapsed time while pipeline runs |
| `useHistory` | Load project list, sessions, session details from storage |
| `useLiveSessionStatus` | Poll for active sessions across all projects |
| `useMcpRuntime` | Check MCP server runtime statuses per CLI |
| `useMcpServers` | MCP server CRUD, enable/disable, CLI bindings |
| `usePipeline` | Pipeline lifecycle: start, pause, resume, cancel, answer |
| `usePipelineEvents` | Subscribe to all Tauri pipeline events, update state |
| `useRecentTerminal` | Derive most recent terminal output with auto-scroll |
| `useSettings` | Load and persist app settings via Tauri commands |
| `useSkills` | Skills catalogue CRUD and refresh |
| `useUpdateCheck` | App update checking (every 4 hours or on focus) |
| `useWorkspace` | Workspace folder selection via native dialog |

#### Types (`src/types/`)

All re-exported from `index.ts`. Domain files:

- `agents.ts` — `AgentRole`, `AgentBackend`, `CliHealth`, `CliStatus`, `CliVersionInfo`, `AllCliVersions`
- `events.ts` — `PIPELINE_EVENTS` constant, all pipeline event types, `RunEvent` variants, `RunStatus`, `StageEndStatus`
- `history.ts` — `RunOptions`, `PipelineRequest`, `WorkspaceInfo`, `SessionDetail`, `RunDetail`, `QuestionEntry`
- `mcp.ts` — `McpServer`, CRUD payloads, `McpRuntimeStatus`, `McpCliRuntimeStatus`
- `navigation.ts` — `ActiveView` union
- `pipeline.ts` — `PipelineStage`, `StageStatus`, `JudgeVerdict`, `PipelineStatus`, `StageResult`, `Iteration`, `PipelineRun`
- `settings.ts` — `AppSettings`, `DEFAULT_SETTINGS`, `CLI_MODEL_OPTIONS`
- `skills.ts` — `Skill`, create/update payloads
- `storage.ts` — `SettingsFile`, `ProjectEntry`, `SkillFile`, `McpServerConfig`, `SessionMeta`, `RunSummaryFile`, `GitBaseline`, `ReviewFindings`, `ChatMessage`

#### Utilities (`src/utils/`)

- `statusHelpers.ts` — Pipeline/run status predicates (`isActive`, `isTerminal`, `isRunInProgress`) and tone/colour mappers
- `formatters.ts` — Display formatting (timestamps, durations, tokens, costs, folder names, text truncation)
- `planParser.ts` — Extract clean plan text from noisy CLI output
- `cliParser.ts` — Parse Claude CLI JSON output into structured results
- `agentSettings.ts` — Agent assignment validation, minimum-fields checks, stage-to-backend bindings
- `stageModelLabels.ts` — Get configured model label for a pipeline stage

### Desktop Backend

- Entry: `src-tauri/src/main.rs` → `lib.rs` (app init, migrations, command registration, plugin setup).
- Top-level modules: `events.rs` (IPC event payloads), `git.rs` (git helpers: repo detection, status, diff, branch).

#### Commands (`src-tauri/src/commands/`)

| Module | Purpose |
|--------|---------|
| `mod.rs` | `AppState` definition (cancel/pause flags, answer channels) |
| `app.rs` | `has_live_sessions()` — check for active sessions |
| `workspace.rs` | Workspace selection and validation |
| `pipeline.rs` | `run_pipeline`, `cancel_pipeline`, `pause_pipeline`, `resume_pipeline`, `answer_pipeline_question` |
| `settings.rs` | `get_settings`, `save_settings` |
| `skills.rs` | Skills CRUD commands |
| `mcp.rs` | MCP server CRUD, CLI bindings, API key setup |
| `mcp_runtime/` | MCP runtime status checking and fix operations (`mod.rs`, `install.rs`, `native.rs`, `parse.rs`) |
| `cli.rs` | `check_cli_health` — verify CLI binaries are available |
| `cli_version.rs` | Version checking and update command generation |
| `cli_http.rs` | HTTP utilities for version fetching |
| `cli_util.rs` | CLI path resolution and validation |
| `git_bash.rs` | Windows-only Git Bash detection and invocation |
| `history.rs` | Session/run history: list projects/sessions, get session detail, create session, delete session |

#### Agents (`src-tauri/src/agents/`)

Pluggable CLI-based agent runner for 5 backends:

- `base/mod.rs` — Core `run_cli_agent()`: async process spawning, stdout/stderr streaming, event emission
- `base/windows.rs` — Windows-specific: Git Bash integration, temp prompt files, process tree termination
- `claude.rs` — Claude Code CLI (`--print --verbose --output-format stream-json`)
- `codex.rs` — OpenAI Codex CLI (`exec --full-auto`)
- `gemini.rs` — Google Gemini CLI (`--approval-mode yolo`)
- `kimi.rs` — Kimi CLI (`--print --output-format stream-json`, PYTHONIOENCODING=utf-8)
- `opencode.rs` — OpenCode CLI (stdin prompt)
- `mcp.rs` — MCP config builder for temporary server configs

#### Orchestrator (`src-tauri/src/orchestrator/`)

Multi-stage AI pipeline with self-improving iteration loop:

- `pipeline/mod.rs` — Main `run_pipeline()`: setup → iterate until COMPLETE
- `pipeline/direct_task.rs` — Single-shot agent execution (bypasses full pipeline)
- `iteration/mod.rs` — Single iteration: enhance → skill select → plan → code → review → fix → judge
- `iteration/generate.rs` — Coder stage execution
- `iteration/prompt_enhance.rs` — Prompt enhancement stage
- `iteration/stages.rs` — Stage result tracking
- `iteration_planning/mod.rs` — Parallel planning: 1–3 planners, plan audit, merging
- `iteration_planning/persistence.rs` — Plan persistence and recovery
- `iteration_review/mod.rs` — Parallel review: 1–3 reviewers, review merger
- `iteration_review/stages.rs` — Review stage execution
- `iteration_review/judge.rs` — Judge agent: COMPLETE vs NOT COMPLETE verdict
- `helpers.rs` — `dispatch_agent()` routing, event emission helpers
- `stages/mod.rs` — Low-level stage execution, cancellation/pause handling
- `stages/execution.rs` — Stage execution helpers
- `run_setup/mod.rs` — `PipelineRun` type, setup/teardown, event emission
- `run_setup/persistence.rs` — Run metadata and event persistence
- `parsing/mod.rs` — Output parsing infrastructure
- `parsing/plan.rs` — Plan audit verdict parsing
- `parsing/reviewer.rs` — Review findings parsing
- `session_memory.rs` — Session history context building
- `skill_selection.rs` — Skill selector agent
- `skill_stage.rs` — Skill stage integration
- `plan_gate.rs` — Plan approval gate (user prompt on failed audit)
- `context_summary.rs` — Executive context summary, diff after coder/fixer
- `user_questions.rs` — User question handling during pipeline
- `prompts/` — Prompt templates: `enhancer.rs`, `planner.rs`, `plan_auditor.rs`, `generator.rs`, `reviewer.rs`, `review_merger.rs`, `fixer.rs`, `judge.rs`, `executive_summary.rs`, `skills.rs`

#### Models (`src-tauri/src/models/`)

| Module | Key types |
|--------|-----------|
| `agents.rs` | `AgentRole`, `AgentBackend`, default path/model helpers |
| `pipeline.rs` | `PipelineStage`, `StageStatus`, `JudgeVerdict`, `PipelineStatus`, `StageResult`, `Iteration` |
| `events.rs` | `RunStatus`, `StageEndStatus`, `PlanAuditVerdict`, `RunEvent` enum |
| `questions.rs` | `PipelineRequest`, `PipelineQuestion`, `PipelineAnswer` |
| `settings.rs` | `AppSettings` struct (all config fields) |
| `skills.rs` | `Skill` record |
| `storage.rs` | `ProjectEntry`, `SessionMeta`, `RunSummary`, `GitBaseline`, `ReviewFindings`, `ChatMessage` |
| `environment.rs` | `WorkspaceInfo`, `CliStatus`, `CliHealth`, `CliVersionInfo`, `AllCliVersions` |
| `mcp.rs` | Frontend-facing `McpServer` |
| `mcp_runtime.rs` | `McpVerificationConfidence`, `McpRuntimeStatus`, `McpCliRuntimeStatus` |

#### Storage (`src-tauri/src/storage/`)

File-based persistence with atomic writes (.tmp → .bak → final rename) and per-file-type mutex locks.

| Module | Purpose |
|--------|---------|
| `mod.rs` | `config_dir()` (→ `~/.ea-code/`), `atomic_write()`, backup recovery, file locks, `now_rfc3339()` |
| `index.rs` | Fast session→project, run→session lookups (`index.json`) |
| `projects.rs` | Project CRUD (`projects.json`) |
| `sessions.rs` | Session CRUD (`projects/{pid}/sessions/{sid}/session.json`) |
| `runs/mod.rs` | Run lifecycle: create, read, update, list |
| `runs/events.rs` | Append-only event JSONL (`events.jsonl`) |
| `runs/git.rs` | Git baseline capture (branch, commit, status) |
| `settings.rs` | App settings (`settings.json`) |
| `skills.rs` | Skills CRUD (`skills/{id}.json`) |
| `mcp.rs` | MCP config (`mcp.json`): servers and CLI bindings |
| `messages.rs` | Chat message storage for sessions |
| `migration.rs` | Data migration: flat → hierarchical project/session layout |
| `recovery.rs` | Crash recovery: mark interrupted runs, config dir migration |
| `cleanup.rs` | Retention policy: delete runs older than N days |

#### Reserved Directories

- `src-tauri/src/swarm/` — Empty; reserved for future multi-agent swarm coordination.

### Supported Agent Backends

- `claude` — Claude Code CLI
- `codex` — OpenAI Codex CLI
- `gemini` — Google Gemini CLI
- `kimi` — Kimi CLI (Moonshot)
- `opencode` — OpenCode CLI (Zhipu)

### Pipeline Stages

The orchestrator runs a multi-stage self-improving loop. Full stage list:

1. **PromptEnhance** — Enhancer agent refines the user prompt
2. **SkillSelect** — Skill selector identifies applicable skills
3. **Plan / Plan2 / Plan3** — Up to 3 parallel planners generate strategies
4. **PlanAudit** — Auditor validates plans (APPROVED / REJECTED / NEEDS_REVISION)
5. **Coder** — Coder agent implements changes
6. **DiffAfterCoder** — Captures diff after coding
7. **CodeReviewer / CodeReviewer2 / CodeReviewer3** — Up to 3 parallel reviewers audit code
8. **ReviewMerge** — Merges findings from multiple reviewers
9. **CodeFixer** — Addresses review findings
10. **DiffAfterCodeFixer** — Captures diff after fixing
11. **Judge** — Decides COMPLETE or iterate again
12. **ExecutiveSummary** — Optional final summary
13. **DirectTask** — Single-agent mode (bypasses full pipeline)

### Storage Layout

```
~/.ea-code/
├── settings.json
├── projects.json
├── index.json                            # Fast lookup index
├── mcp.json                              # MCP server configs and CLI bindings
├── projects/
│   └── <project-id>/
│       ├── project.json
│       └── sessions/
│           └── <session-id>/
│               ├── session.json
│               └── runs/
│                   └── <run-id>/
│                       ├── summary.json  # Atomic writes
│                       └── events.jsonl  # Append-only event log
├── skills/
│   └── <skill-id>.json
└── prompts/                              # Temp files for multi-line CLI prompts (Windows)
```

Use the existing migration and recovery helpers under `storage/` rather than inventing a second persistence path.

### Website (`frontend/web/`)

Marketing site: React 19, TypeScript 5.8, Vite 7, Tailwind CSS v4, Lucide React icons.

- Components: `Navbar`, `Hero` (screenshot carousel), `AgentsBar`, `WhySection` (single vs multi-agent comparison), `Pipeline` (10-stage visualisation), `Features` (6-feature grid), `CTA` (platform-specific downloads), `Footer`
- Hooks: `useReleaseInfo` — fetches release metadata from `/api/v1/updates/release-info`
- Styling: custom CSS variables (surface, accent, muted), animations (fade-in-up, pulse-glow, stagger), dot-grid background
- Assets: logos and screenshots in `public/`

### Release and CI

- Release scripts: `scripts/release.sh` (Bash) and `scripts/release.ps1` (PowerShell) — bump version in `tauri.conf.json`, `Cargo.toml`, `package.json`; create git tag; push to origin/main.
- GitHub Actions: `.github/workflows/release.yml` — triggers on `v*` tags; builds Windows NSIS installer (macOS DMG currently commented out); signs with `TAURI_SIGNING_PRIVATE_KEY`.

## Pipeline and IPC Conventions

- Frontend-to-backend calls use `invoke(...)` with camelCase argument names.
- Tauri event names are colon-delimited and must match exactly in Rust and TypeScript:
  - `pipeline:started`
  - `pipeline:stage`
  - `pipeline:log`
  - `pipeline:artifact`
  - `pipeline:question`
  - `pipeline:completed`
  - `pipeline:error`
- Rust enums that cross IPC serialise as `snake_case`; matching TypeScript string literal unions must stay in sync.
- Struct payloads exposed to the frontend should use `#[serde(rename_all = "camelCase")]`.
- Timestamps are RFC 3339 strings produced via `storage::now_rfc3339()`.
- Cancellation and pause flow use `Arc<AtomicBool>` flags stored in `commands::AppState`; preserve that pattern.
- User questions during pipeline use one-shot channels per run in `AppState`.

## Coding Standards

### General

- Match surrounding code before introducing a new pattern.
- Use British English in comments, docs, and user-facing text.
- Do not leave placeholder code or TODO comments.

### TypeScript and React

- `strict`, `noUnusedLocals`, and `noUnusedParameters` are enabled.
- Use functional components and explicit prop interfaces.
- Prefer named exports.
- Use `const` by default.
- Tailwind CSS v4 utility classes are the default styling approach.
- Avoid inline `style={}` unless there is a concrete need.

### Rust

- Prefer `Result<T, String>` for Tauri commands.
- Avoid `unwrap()` on fallible runtime paths.
- Keep serde naming aligned with the frontend contract.
- Use the existing modular orchestrator layout rather than rebuilding large single-file flows.

## Do Not Touch Casually

- Generated files under `frontend/desktop/src-tauri/gen/`
- Build output under `frontend/desktop/src-tauri/target/`
- Lockfiles unless the change genuinely requires them
- `.claude/skills/` unless the task is specifically about repo-local skills
- Reserved `src-tauri/src/swarm/` directory

## Quick Reference

- Desktop frontend: `frontend/desktop/src/`
- Desktop backend: `frontend/desktop/src-tauri/src/`
- Website: `frontend/web/src/`
- Release scripts: `scripts/release.ps1`, `scripts/release.sh`
- CI workflow: `.github/workflows/release.yml`
- Root docs: `README.md`, `AGENTS.md`, `CLAUDE.md`

## What Not To Do

- Do not commit unless the user explicitly asks.
- Do not add dependencies without justification.
- Do not hand-edit generated output when a source file should be changed instead.
- Do not treat old SQLite or `~/.config/ea-code/` references as the current architecture.
