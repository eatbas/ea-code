# CLAUDE.md - EA Code Agent Instructions

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
- Rust agent execution helpers live in `frontend/desktop/src-tauri/src/agents/base/`.
- Rust models are split by domain in `frontend/desktop/src-tauri/src/models/`.
- File persistence helpers live in `frontend/desktop/src-tauri/src/storage/`.

## Current Architecture

### Desktop Frontend

- Entry points: `frontend/desktop/src/main.tsx` and `frontend/desktop/src/App.tsx`
- Main UI lives in `frontend/desktop/src/components/`
- Feature folders already in use include `AgentsView/`, `CliSetupView/`, `McpView/`, and `shared/`

### Desktop Backend

- Tauri commands: `frontend/desktop/src-tauri/src/commands/`
- Agent runners: `frontend/desktop/src-tauri/src/agents/`
- Orchestrator pipeline: `frontend/desktop/src-tauri/src/orchestrator/`
- File storage: `frontend/desktop/src-tauri/src/storage/`
- Shared event payloads: `frontend/desktop/src-tauri/src/events.rs`
- Shared Rust models: `frontend/desktop/src-tauri/src/models/`

### Supported Agent Backends

- `claude`
- `codex`
- `gemini`
- `kimi`
- `opencode`

### Storage Layout

- Base directory: `~/.ea-code/`
- Index file: `index.json`
- Project list: `projects.json`
- Sessions: `projects/<project-id>/sessions/<session-id>/`
- Runs: stored inside each session directory with `summary.json`, `events.jsonl`, git metadata, and artefacts
- Skills: `skills/`
- Prompt temp files: `prompts/`

Use the existing migration and recovery helpers under `storage/` rather than inventing a second persistence path.

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

## Quick Reference

- Desktop frontend: `frontend/desktop/src/`
- Desktop backend: `frontend/desktop/src-tauri/src/`
- Website: `frontend/web/src/`
- Release scripts: `scripts/release.ps1`, `scripts/release.sh`
- Root docs: `README.md`, `AGENTS.md`, `CLAUDE.md`

## What Not To Do

- Do not commit unless the user explicitly asks.
- Do not add dependencies without justification.
- Do not hand-edit generated output when a source file should be changed instead.
- Do not treat old SQLite or `~/.config/ea-code/` references as the current architecture.
