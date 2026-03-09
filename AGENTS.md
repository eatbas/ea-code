# AGENTS.md - Guidance for AI Agents Working on EA Code

This file provides instructions for any AI agent (Claude Code, Codex, Gemini, Copilot, etc.) that modifies this codebase.

---

## Golden Rules

1. **Verify before delivering.** Every change must pass the relevant build check before being presented to the user.
2. **Keep files small.** No file exceeds 300 lines. Split proactively.
3. **Reuse, do not repeat.** Extract shared logic into hooks, components, or modules.
4. **Match existing patterns.** Read surrounding code before writing new code.

---

## Scope Selection (Important)

- **Default scope is desktop app code.** Start in `frontend/desktop/` unless the user explicitly asks for website work.
- **Do not inspect `frontend/web/`** unless the request clearly says `website`, `web`, or points to a path under `frontend/web/`.
- If the request is ambiguous, assume desktop scope and state that assumption briefly.

---

## Build Verification Checklist

### After touching Rust code (`frontend/desktop/src-tauri/**/*.rs`)

```sh
cd frontend/desktop/src-tauri && cargo check
```

If `cargo check` fails, fix **all** errors before proceeding. Do not present broken code.

### After touching desktop TypeScript/React (`frontend/desktop/src/**/*.{ts,tsx}`)

```sh
cd frontend/desktop && npx tsc --noEmit
```

If `tsc` fails, fix **all** errors before proceeding. Do not present broken code.

### After touching website TypeScript/React (`frontend/web/src/**/*.{ts,tsx}`)

```sh
cd frontend/web && npx tsc --noEmit
```

If `tsc` fails, fix **all** errors before proceeding. Do not present broken code.

### After touching both desktop Rust and desktop TypeScript

Run both desktop checks.

---

## File Size Policy

**Hard limit: 300 lines per file.**

When a file grows beyond this:

### Rust

Split into a directory module:

```text
# Before
frontend/desktop/src-tauri/src/orchestrator.rs  (500+ lines)

# After
frontend/desktop/src-tauri/src/orchestrator/
|- mod.rs           # Re-exports public API
|- pipeline.rs      # Pipeline loop logic
`- stages.rs        # Individual stage handlers
```

Update `mod.rs` to re-export so callers are unaffected:

```rust
mod pipeline;
mod stages;

pub use pipeline::*;
pub use stages::*;
```

### TypeScript / React

Split into a component folder:

```text
# Before
frontend/desktop/src/components/Sidebar.tsx  (400+ lines)

# After
frontend/desktop/src/components/Sidebar/
|- index.tsx          # Main Sidebar component
|- SessionList.tsx    # Session list sub-component
`- ProjectPicker.tsx  # Project picker sub-component
```

---

## Code Reuse Requirements

### Do Not Duplicate

- **UI patterns** - If two components share similar markup (buttons, cards, status badges), extract a shared component into `frontend/desktop/src/components/shared/`.
- **Hooks** - Shared state logic belongs in `frontend/desktop/src/hooks/`. Compose small hooks rather than creating monolithic ones.
- **Rust utilities** - Common helpers go in dedicated modules. Agent adapters must implement the `AgentRunner` trait.
- **Types** - Frontend types are defined once in `frontend/desktop/src/types/index.ts`. Rust equivalents live in `frontend/desktop/src-tauri/src/models/`. Keep them synchronised.

### Before Creating a New File

1. Search the codebase for existing code that does something similar.
2. If found, extend or refactor the existing code.
3. Only create a new file if no suitable location exists.

---

## Coding Standards

### Language

- **British English** everywhere: comments, docs, variable names where applicable (e.g., `colour`, `behaviour`, `initialise`).

### TypeScript

- `strict: true` - no `any`, no implicit returns, no unused variables.
- Functional components with explicit prop interfaces.
- Named exports preferred over default exports.
- `const` by default; `let` only when mutation is required.

### Rust

- `#[serde(rename_all = "camelCase")]` on all structs exposed to the frontend.
- Proper error handling: `Result<T, String>` for Tauri commands.
- No `unwrap()` on fallible operations in production code paths - use `?` or explicit error handling.
- Tauri command parameters must use camelCase to match frontend `invoke()` calls.

### CSS

- Tailwind CSS v4 utility classes only. No inline `style={}` attributes.

---

## Architecture Awareness

### Frontend -> Backend Communication

- Frontend calls Rust via `invoke("command_name", { paramName: value })`.
- Backend emits events to frontend via Tauri's event system.
- All parameters are serialised as camelCase JSON.

### Database

- SQLite via Diesel ORM 2.2.
- Schema defined in `frontend/desktop/src-tauri/migrations/`.
- Auto-generated schema: `frontend/desktop/src-tauri/src/schema.rs` - **do not edit manually**.
- Models: `frontend/desktop/src-tauri/src/db/models/` (Diesel), `frontend/desktop/src-tauri/src/models/` (Tauri command payloads).

### Pipeline

- `frontend/desktop/src-tauri/src/orchestrator/` drives the generate -> diff -> review -> fix -> judge loop.
- Agents are invoked via `tokio::process::Command` (async).
- Cancellation uses `Arc<AtomicBool>` checked between stages.

---

## What NOT to Do

- **Do not commit** unless the user explicitly requests it.
- **Do not add dependencies** without justification.
- **Do not leave TODO comments** - implement the full solution or flag it to the user.
- **Do not write placeholder code** like `// ... rest of code` or `unimplemented!()`.
- **Do not modify `.gitignore`, `Cargo.lock`, or `package-lock.json`** without reason.
- **Do not modify `schema.rs`** directly - it is auto-generated by Diesel.

---

## Quick Reference: Key File Locations

| Purpose | Path |
|---|---|
| Desktop frontend types | `frontend/desktop/src/types/index.ts` |
| Desktop React hooks | `frontend/desktop/src/hooks/` |
| Desktop UI components | `frontend/desktop/src/components/` |
| Website frontend root | `frontend/web/src/` |
| Tauri commands | `frontend/desktop/src-tauri/src/commands/` |
| Rust models (serde) | `frontend/desktop/src-tauri/src/models/` |
| DB models (Diesel) | `frontend/desktop/src-tauri/src/db/models/` |
| DB queries | `frontend/desktop/src-tauri/src/db/{table}.rs` |
| Agent adapters | `frontend/desktop/src-tauri/src/agents/` |
| Pipeline engine | `frontend/desktop/src-tauri/src/orchestrator/` |
| Diesel schema | `frontend/desktop/src-tauri/src/schema.rs` (auto-generated) |
| Migrations | `frontend/desktop/src-tauri/migrations/` |
| MCP server | `frontend/desktop/src-tauri/src/bin/mcp_server/` |
