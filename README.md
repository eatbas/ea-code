# EA Code

EA Code is a desktop control room for AI coding CLIs.

Instead of relying on one model to plan, code, review, and judge its own work, EA Code lets you wire different agents into each stage of the job. You choose the project, assign the roles, run the pipeline, and watch the result move from prompt to reviewed output with full session history.

Website: [ea-code.atbas.xyz](https://ea-code.atbas.xyz)

## What It Does

- Runs coding tasks through a multi-stage orchestration pipeline
- Mixes and matches CLI backends per role
- Keeps project threads, run logs, artefacts, and chat history locally
- Lets you maintain reusable skills and MCP connections for better context
- Supports both full review loops and quicker direct-task or budget-mode runs

Supported backends in the current desktop app:

- Claude
- Codex
- Gemini
- Kimi
- OpenCode

## Why EA Code

Most AI coding tools collapse everything into a single conversation. EA Code takes a different approach:

- one agent can sharpen the prompt
- several agents can propose plans in parallel
- another agent can audit the plan before code is written
- separate reviewers can critique the implementation
- a fixer can apply the review feedback
- a final judge decides whether the task is complete or should loop again

That gives you a more inspectable workflow, better separation of responsibilities, and a clearer record of how a result was produced.

## How The Pipeline Works

The exact path depends on settings, but a typical run looks like this:

1. Prompt enhancement clarifies the task.
2. Skill selection pulls in relevant local guidance.
3. One or more planners draft an implementation plan.
4. A plan auditor pressure-tests that plan.
5. The coder makes the change.
6. Up to three reviewers inspect the result.
7. A review merge stage combines reviewer feedback.
8. The fixer applies the required changes.
9. The judge decides whether the result is complete.
10. An executive summary records what happened.

If the judge is not satisfied, EA Code can loop through another iteration with the previous context attached.

## Desktop App Highlights

- Project picker for working across multiple repositories
- Session-based history so each task stays grouped in its own thread
- Live run timeline with stage logs and saved artefacts
- Agent and model assignment per stage
- CLI health checks, version checks, and in-app update actions
- Skill editor for reusable instructions
- MCP configuration view for project context integrations
- Pause, resume, cancel, and plan-approval controls during runs

## Tech Stack

- Desktop shell: [Tauri v2](https://tauri.app/)
- Frontend: React 19, TypeScript, Vite, Tailwind CSS v4
- Backend: Rust with Tokio
- Local persistence: file-based storage under `~/.ea-code/`

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) LTS
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/)
- At least one supported AI CLI installed and available on your machine

### Run The Desktop App

```bash
cd frontend/desktop
npm install
npm run tauri dev
```

### Build A Production Desktop App

```bash
cd frontend/desktop
npm run tauri build
```

## Local Data

EA Code stores its local state in `~/.ea-code/`, including:

- settings
- skills
- project and session metadata
- run summaries, events, and artefacts
- prompt files used during orchestration

The repository does not require a hosted backend to run the desktop product.

## Repo Layout

```text
frontend/
`-- desktop/
    |-- src/          # React UI
    `-- src-tauri/    # Rust backend and orchestration engine

docs/                 # Supporting project documentation
scripts/              # Utility scripts
```

## Development Checks

If you change desktop code, run the matching verification step before shipping it:

```bash
# Desktop TypeScript
cd frontend/desktop
npx tsc --noEmit

# Rust backend
cd frontend/desktop/src-tauri
cargo check
```

## Licence

MIT
