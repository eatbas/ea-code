# CLAUDE.md — EA Code Project Instructions

## Project Overview

Tauri v2 desktop application that orchestrates Claude, Codex, and Gemini CLIs in a self-improving dev loop.

- **Frontend**: React 19 + TypeScript 5.8 + Tailwind CSS v4
- **Backend**: Rust (Tauri v2) + SQLite (Diesel ORM 2.2)
- **Database**: `~/.config/ea-code/ea-code.db`

---

## Mandatory Build Verification

### Rust Backend

After **any** change to files under `src-tauri/`, run:

```sh
cd src-tauri && cargo check
```

Do **not** deliver code that fails `cargo check`. If the check fails, fix every error before presenting the result.

### TypeScript Frontend

After **any** change to files under `src/`, run:

```sh
npx tsc --noEmit
```

Do **not** deliver code that fails `tsc --noEmit`. If the check fails, fix every error before presenting the result.

---

## File Size Limit (300 Lines)

No single source file should exceed **300 lines** of code (excluding blank lines and comments as a rough guide — use total line count in practice).

When a file approaches or exceeds this limit:

1. **Identify logical boundaries** — group related functions, types, or components.
2. **Extract into a subfolder module** — e.g., `orchestrator.rs` → `orchestrator/mod.rs` + `orchestrator/pipeline.rs` + `orchestrator/stages.rs`.
3. **Re-export** the public API from `mod.rs` so callers are unaffected.
4. **Frontend equivalent** — split large components into a folder: `Component/index.tsx` + `Component/SubPart.tsx`.

### Currently Over Limit

All files are currently within the 300-line limit after the v0.2.0 refactoring.

---

## Code Reuse & DRY

- **Components**: Extract reusable UI elements into `src/components/shared/` or co-located files. Never duplicate JSX across views.
- **Hooks**: Shared stateful logic belongs in `src/hooks/`. Prefer composition of small hooks over monolithic ones.
- **Rust**: Common utilities go in dedicated modules. Agent adapters share the `AgentRunner` trait from `agents/base.rs`.
- **Types**: Frontend types live in `src/types/` (split by domain, re-exported from `index.ts`). Rust models in `src-tauri/src/models/` (split by domain, re-exported from `mod.rs`). Keep them in sync.

---

## Coding Standards

### General

- **British English** in all comments, documentation, and user-facing text (e.g., "optimise", "behaviour", "colour").
- **Explicit types** — no `any` in TypeScript, no unnecessary `unwrap()` in Rust.
- **No placeholder comments** — never write `// ... rest of code` or `// TODO: implement`. Output complete, functional code.
- **Idiomatic code** — use language-native patterns, but avoid overly clever one-liners that hinder debugging.

### TypeScript / React

- Strict mode enabled (`strict: true` in tsconfig).
- Functional components only. Use hooks for state and side effects.
- Props must have explicit interface definitions — no inline anonymous types.
- Prefer named exports over default exports.
- Use `const` by default; `let` only when reassignment is required.

### Rust

- `#[serde(rename_all = "camelCase")]` on all structs exposed to the frontend.
- Tauri command parameter names must match camelCase in frontend `invoke()` calls.
- Handle errors with `Result<T, String>` for Tauri commands (or a proper error enum).
- Use `Arc<AtomicBool>` pattern for cancellation (see `AppState`).

### CSS / Styling

- Tailwind CSS v4 utility classes. No inline `style={}` unless absolutely necessary.
- Consistent spacing and colour tokens from the Tailwind theme.

---

## Architecture Quick Reference

```
src/                          # Frontend (React + TS)
├── components/               # UI components (keep < 300 lines each)
│   ├── shared/               # Reusable form inputs, constants
│   └── AgentsView/           # Split component folder
├── hooks/                    # Custom React hooks
├── types/                    # Shared type definitions (split by domain)
├── utils/                    # Pure helper functions
├── App.tsx                   # Root layout and routing
└── main.tsx                  # Entry point

src-tauri/                    # Backend (Rust)
├── src/
│   ├── agents/               # CLI adapters (base trait + claude/codex/gemini)
│   ├── bin/mcp_server/       # MCP server binary (split into submodules)
│   ├── commands/             # Tauri IPC commands (split by domain)
│   ├── db/                   # Diesel ORM layer (models, queries per table)
│   ├── models/               # Shared Rust types (split by domain)
│   ├── orchestrator/         # Pipeline engine (split into submodules)
│   ├── schema.rs             # Diesel schema (auto-generated)
│   └── lib.rs                # Tauri app builder
└── migrations/               # Diesel SQL migrations
```

---

## Version Control

- **No commits** from agents unless the user explicitly requests one.
- Verify that generated files (secrets, configs, data) are excluded via `.gitignore`.

---

## Dependencies

When adding or updating dependencies:

- **Rust**: Add to `src-tauri/Cargo.toml`, then run `cargo check`.
- **JS**: Add via `npm install`, then run `npx tsc --noEmit`.
- Prefer well-maintained, widely-used crates/packages. Justify new dependencies.
