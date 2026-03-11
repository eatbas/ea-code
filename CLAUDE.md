# CLAUDE.md - EA Code Project Instructions

## Project Overview

Tauri v2 desktop application that orchestrates Claude, Codex, and Gemini CLIs in a self-improving development loop, plus a separate marketing website.

- **Desktop App**: React 19 + TypeScript 5.8 + Tailwind CSS v4 (Tauri v2 frontend)
- **Website**: React 19 + TypeScript 5.8 + Tailwind CSS v4 (Vite SPA)
- **Backend**: Rust (Tauri v2) + SQLite (Diesel ORM 2.2)
- **Database**: `~/.config/ea-code/ea-code.db`

---

## Scope Rules (Desktop First)

- Default to `frontend/desktop/` for investigation and edits.
- Do **not** inspect `frontend/web/` unless the user explicitly says `website`, `web`, or asks for work under that path.
- If the request is not explicit, assume desktop scope.

---

## Mandatory Build Verification

### Rust Backend

After **any** change to files under `frontend/desktop/src-tauri/`, run:

```sh
cd frontend/desktop/src-tauri && cargo check
```

Do **not** deliver code that fails `cargo check`. If the check fails, fix every error before presenting the result.

### Desktop TypeScript Frontend

After **any** change to files under `frontend/desktop/src/`, run:

```sh
cd frontend/desktop && npx tsc --noEmit
```

Do **not** deliver code that fails `tsc --noEmit`. If the check fails, fix every error before presenting the result.

### Website TypeScript Frontend

After **any** change to files under `frontend/web/src/`, run:

```sh
cd frontend/web && npx tsc --noEmit
```

Do **not** deliver code that fails `tsc --noEmit`. If the check fails, fix every error before presenting the result.

---

## File Size Limit (300 Lines)

No single source file should exceed **300 lines** of code (excluding blank lines and comments as a rough guide - use total line count in practice).

When a file approaches or exceeds this limit:

1. **Identify logical boundaries** - group related functions, types, or components.
2. **Extract into a subfolder module** - e.g., `orchestrator.rs` -> `orchestrator/mod.rs` + `orchestrator/pipeline.rs` + `orchestrator/stages.rs`.
3. **Re-export** the public API from `mod.rs` so callers are unaffected.
4. **Frontend equivalent** - split large components into a folder: `Component/index.tsx` + `Component/SubPart.tsx`.

---

## Code Reuse and DRY

- **Components**: Extract reusable UI elements into `frontend/desktop/src/components/shared/` or co-located files. Never duplicate JSX across views.
- **Hooks**: Shared stateful logic belongs in `frontend/desktop/src/hooks/`. Prefer composition of small hooks over monolithic ones.
- **Rust**: Common utilities go in dedicated modules. Agent adapters share the `AgentRunner` trait from `frontend/desktop/src-tauri/src/agents/base.rs`.
- **Types**: Frontend types live in `frontend/desktop/src/types/` (split by domain, re-exported from `index.ts`). Rust models live in `frontend/desktop/src-tauri/src/models/` (split by domain, re-exported from `mod.rs`). Keep them in sync.

---

## Coding Standards

### General

- **British English** in comments, documentation, and user-facing text (e.g., "optimise", "behaviour", "colour").
- **Explicit types** - no `any` in TypeScript, no unnecessary `unwrap()` in Rust.
- **No placeholder comments** - never write `// ... rest of code` or `// TODO: implement`.
- **Idiomatic code** - use language-native patterns, but avoid unclear one-liners that hinder debugging.

### TypeScript / React

- Strict mode enabled (`strict: true` in tsconfig).
- Functional components only. Use hooks for state and side effects.
- Props must have explicit interface definitions - no inline anonymous types.
- Prefer named exports over default exports.
- Use `const` by default; `let` only when reassignment is required.

### Rust

- `#[serde(rename_all = "camelCase")]` on all structs exposed to the frontend.
- Tauri command parameter names must match camelCase in frontend `invoke()` calls.
- Handle errors with `Result<T, String>` for Tauri commands (or a proper error enum).
- Use `Arc<AtomicBool>` pattern for cancellation.

### CSS / Styling

- Tailwind CSS v4 utility classes.
- No inline `style={}` unless absolutely necessary.

---

## Architecture Quick Reference

```text
frontend/
|- desktop/                      # Tauri desktop app
|  |- src/                       # React frontend (TS)
|  |  |- components/             # UI components
|  |  |  `- shared/              # Reusable UI pieces
|  |  |- hooks/                  # Custom React hooks
|  |  |- types/                  # Shared type definitions
|  |  `- utils/                  # Pure helper functions
|  `- src-tauri/                 # Rust backend
|     |- src/
|     |  |- agents/              # CLI adapters
|     |  |- bin/mcp_server/      # MCP server binary
|     |  |- commands/            # Tauri IPC commands
|     |  |- db/                  # Diesel ORM layer
|     |  |- models/              # Shared Rust types
|     |  |- orchestrator/        # Pipeline engine
|     |  `- schema.rs            # Diesel schema (auto-generated)
|     `- migrations/             # Diesel SQL migrations
`- web/                          # Marketing website (only inspect when requested)
   `- src/
```

---

## Type Synchronisation (Critical)

Rust enums and TypeScript types **must stay in sync** when adding or renaming pipeline stages, agent types, or any shared enum.

- Rust: `#[serde(rename_all = "snake_case")]` on enums → `PipelineStage::PromptEnhance` serialises as `"prompt_enhance"`.
- TypeScript: Matching string literal types → `"prompt_enhance"`.
- A mismatch silently breaks IPC at runtime. Always update both sides together.

**Two model layers in Rust:**
- `src/db/models/` — Diesel derives (`Queryable`, `Selectable`, `Insertable`). These map directly to DB rows.
- `src/models/` — Serde derives (`Serialize`, `Deserialize`). These are the Tauri command payloads sent to the frontend.

---

## Timestamp & Event Conventions

- **Timestamps** are stored as RFC 3339 strings (e.g., `"2026-03-11T14:30:00Z"`), not integers. Use the `now_rfc3339()` helper.
- **Event names** must be identical snake_case strings in both Rust (`app_handle.emit("pipeline_progress", ...)`) and TypeScript (`listen("pipeline_progress", ...)`).
- **Migrations** are embedded in the binary via `embed_migrations!()` — no runtime SQL files needed.

---

## Development Environment

- Vite dev server: port **1420** (fixed in `tauri.conf.json`), HMR on port **1421**.
- No `.env` loading — database and config paths are hardcoded to `~/.config/ea-code/`.
- Prompt temp files written to `~/.config/ea-code/prompts/` (Windows Git Bash workaround for multi-line args).

---

## Version Control

- **No commits** from agents unless the user explicitly requests one.
- Verify that generated files (secrets, configs, data) are excluded via `.gitignore`.

---

## Dependencies

When adding or updating dependencies:

- **Rust**: Add to `frontend/desktop/src-tauri/Cargo.toml`, then run `cargo check`.
- **Desktop JS**: `cd frontend/desktop && npm install`, then run `npx tsc --noEmit`.
- **Web JS**: `cd frontend/web && npm install`, then run `npx tsc --noEmit`.
- Prefer well-maintained, widely-used crates/packages. Justify new dependencies.
