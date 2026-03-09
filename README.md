# EA Code

A desktop application that orchestrates multiple AI CLIs — Claude, Codex, Gemini, Kimi, and OpenCode — in a self-improving development loop. Built with [Tauri v2](https://tauri.app/), [React 19](https://react.dev/), [Tailwind CSS v4](https://tailwindcss.com/), and [Rust](https://www.rust-lang.org/).

## How It Works

EA Code runs your coding tasks through a **9-stage pipeline** that iterates until the result meets quality standards:

1. **Prompt Enhance** — Refines natural language into precise instructions
2. **Skill Select** — Matches the request against a curated skills catalogue
3. **Plan** — Generates a step-by-step execution plan with file targets
4. **Plan Audit** — Reviews the plan for gaps, risks, and edge cases
5. **Generate** — Writes or modifies code per the approved plan
6. **Review** — Automated code review (BLOCKER / WARNING / NIT severity)
7. **Fix** — Applies review suggestions
8. **Judge** — Final verdict: COMPLETE or loop back (up to 3 iterations by default)
9. **Executive Summary** — Concise report of all changes

Each pipeline role can be assigned to any supported AI backend, so you can mix and match agents per stage.

## Features

- **Multi-agent orchestration** — Assign Claude, Codex, Gemini, Kimi, or OpenCode to each pipeline role independently
- **Self-improving loop** — The judge stage can send work back through the pipeline with context from previous attempts
- **Plan approval gate** — Optionally pause after planning for human review, with configurable auto-approve timeout
- **Retry with augmentation** — Failed agent calls are retried with "PREVIOUS ATTEMPT FAILED" context injection
- **Skills system** — Persist domain-specific knowledge (framework patterns, coding conventions) that agents can reference
- **MCP integration** — Agents access project history via Model Context Protocol servers
- **Session tracking** — Full history of runs, iterations, stages, logs, and artefacts per workspace
- **Direct task mode** — Bypass the pipeline and call a single agent directly
- **Built-in auto-updater** — Checks for new releases automatically

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS recommended)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri v2 prerequisites](https://tauri.app/start/prerequisites/)
- At least one supported AI CLI installed: `claude`, `codex`, `gemini`, `kimi`, or `opencode`

## Getting Started

### Desktop App

```bash
cd frontend/desktop

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production (creates installer)
npm run tauri build
```

### Marketing Website

```bash
cd frontend/web

# Install dependencies
npm install

# Run in development mode
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

#### Docker (Website)

```bash
docker build -t ea-code-web frontend/web/
docker run -p 80:80 ea-code-web
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Desktop framework** | Tauri v2 |
| **Frontend** | React 19, TypeScript 5.8, Tailwind CSS v4, Vite 7 |
| **Backend** | Rust, Tokio (async runtime) |
| **Database** | SQLite with Diesel ORM 2.2 (WAL mode) |
| **Serialisation** | Serde (camelCase for frontend IPC) |
| **Website** | React 19, Vite 7, Tailwind CSS v4, Lucide icons |
| **Website deployment** | Docker (Node 22 Alpine build, Nginx Alpine serve) |

## Project Structure

```
frontend/
├── desktop/                      # Tauri desktop app
│   ├── src/                      # React frontend
│   │   ├── components/           # UI components
│   │   │   ├── shared/           # Reusable form inputs, constants
│   │   │   └── AgentsView/       # Split component folders
│   │   ├── hooks/                # Custom React hooks
│   │   ├── types/                # Shared type definitions
│   │   ├── utils/                # Pure helper functions
│   │   ├── App.tsx               # Root layout and routing
│   │   └── main.tsx              # Entry point
│   └── src-tauri/                # Rust backend
│       ├── src/
│       │   ├── agents/           # CLI adapters (Claude, Codex, Gemini, Kimi, OpenCode)
│       │   ├── bin/mcp_server/   # MCP server binary
│       │   ├── commands/         # Tauri IPC commands
│       │   ├── db/               # Diesel ORM layer
│       │   ├── models/           # Shared Rust types
│       │   ├── orchestrator/     # Pipeline engine (12 modules)
│       │   ├── schema.rs         # Diesel schema (auto-generated)
│       │   └── lib.rs            # Tauri app builder
│       └── migrations/           # Diesel SQL migrations
│
└── web/                          # Marketing website
    ├── src/
    │   ├── components/           # Landing page sections
    │   ├── App.tsx
    │   └── main.tsx
    ├── Dockerfile                # Multi-stage Docker build
    └── package.json
```

## Configuration

All settings are persisted in a SQLite database at `~/.config/ea-code/ea-code.db` and configurable through the app UI:

- **CLI paths** — Custom paths for each AI CLI binary
- **Agent assignments** — Which CLI backs each pipeline role
- **Model selection** — Default and per-role model overrides
- **Iteration control** — Max iterations (default 3), git requirement toggle
- **Plan gate** — Require approval, auto-approve timeout (default 45s), max revisions
- **Retry settings** — Agent retry count, timeout per agent call
- **Agent capability** — Max turns for agentic CLIs that support multi-turn

## Build Verification

After making changes, run the appropriate checks:

```bash
# Rust backend
cd frontend/desktop/src-tauri && cargo check

# Desktop TypeScript
cd frontend/desktop && npx tsc --noEmit

# Website TypeScript
cd frontend/web && npx tsc --noEmit
```

## Licence

MIT
