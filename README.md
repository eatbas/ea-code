<div align="center">

# Maestro

### Stop Paying for Six AI Subscriptions and Only Using One

You're subscribed to Claude, Gemini, Codex, Copilot, Kimi, and OpenCode. But right now, you're copy-pasting between them, switching tabs, and hoping one model gets it right on the first try.

**What if they all worked together instead?**

Maestro is a desktop app that wires your AI coding CLIs into a single orchestrated pipeline — where each agent plays the role it's best at, and the result is better than anything one model could produce alone.

[Website](https://maestro.atbas.xyz) &#8226; [Get Started](#getting-started) &#8226; [How It Works](#how-the-pipeline-works)

</div>

---

## The Problem

You're paying for multiple AI coding subscriptions. You use Claude for planning, Gemini for quick questions, Codex or Copilot for generation. But you're doing the orchestration manually — switching between tools, re-explaining context, and judging the output yourself.

That's expensive, slow, and exhausting.

## The Solution

Maestro puts every AI CLI you already pay for into a structured pipeline:

- **An orchestrator sharpens your prompt** so the task is crystal clear (optional)
- **Up to three agents plan in parallel** — you get multiple perspectives, not one guess
- **Plan Merge consolidates them** into a single coherent plan you approve before any code is written
- **A coder implements it**, then **up to three reviewers critique it independently**
- **Review Merge deduplicates and prioritises** all findings by severity
- **A Code Fixer applies the feedback** — and you can **trigger more review cycles** until you're satisfied

You assign the roles. Claude plans, Codex codes, Gemini reviews — or any combination you want. Each model does what it's best at. The result is code that's been planned, written, reviewed, and fixed by multiple AI agents working together.

## Supported AI Backends

| Backend | CLI |
|---------|-----|
| Claude | Claude Code |
| Codex | OpenAI Codex CLI |
| Copilot | GitHub Copilot CLI |
| Gemini | Google Gemini CLI |
| Kimi | Kimi CLI |
| OpenCode | OpenCode CLI |

Use one, use all six — assign any backend to any stage.

## How The Pipeline Works

```
 Your Prompt
     |
     v
 [1. Orchestrator] ---------> Enhances the prompt and generates a summary title (optional)
     |
     v
 [2. Planner x3] -----------> Up to 3 agents draft plans IN PARALLEL
     |
     v
 [3. Plan Merge] -----------> Consolidates all plans — you approve before coding
     |
     v
 [4. Coder] -----------------> Implements the approved plan
     |
     v
 [5. Reviewer x3] -----------> Up to 3 agents review IN PARALLEL (via git diff)
     |
     v
 [6. Review Merge] ----------> Deduplicates and prioritises findings by severity
     |
     v
 [7. Code Fixer] --+---------> Applies critical and major fixes
                    |
                    +---------> Redo Review? Cycles back to Reviewers.
```

No model marks its own homework — reviewers resume planner sessions for context, and the Code Fixer resumes the coder's session.

## Why This Beats Using One Agent

| Single Agent | Maestro |
|---|---|
| One model plans, codes, reviews, and judges its own work | Different agents specialise in each role |
| Blind spots go unnoticed | Parallel reviewers catch what one misses |
| You manually re-prompt when output is wrong | Redo Review cycles re-review and fix with full context |
| No review before implementation begins | Plan approval gates let you review before any code is written |
| You pay for 6 subscriptions and use 1 at a time | Every subscription earns its keep |

## Desktop App Highlights

- **Project Picker** — switch between repositories instantly
- **Session History** — every task grouped in its own thread with full traceability
- **Live Run Timeline** — watch stages execute with real-time streaming output
- **Agent Assignment** — configure which backend handles which stage
- **Plan Approval Gates** — pause after Plan Merge to review, edit, or provide feedback
- **Redo Review Cycles** — trigger another review + merge + fix cycle as many times as needed
- **Debug Log Viewer** — collapsible real-time pipeline execution trace with one-click copy
- **Pause / Resume / Cancel** — full control during runs
- **CLI Health Checks** — verify agent availability and update CLIs in-app
- **Fully Local** — everything stored on your machine under `~/.maestro/`, no cloud backend required

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) LTS
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri prerequisites](https://tauri.app/start/prerequisites/)
- At least one supported AI CLI installed on your machine
- Initialise the bundled API submodule after cloning:
  ```bash
  git submodule update --init --recursive
  ```

### Platform Notes

- macOS (including Apple Silicon Mac mini): install Xcode Command Line Tools so `clang` is available.
- Windows ARM64 and Windows x64: install LLVM or the Visual Studio C++ Clang tools. Maestro now auto-discovers common clang installs for both architectures during Rust builds.
- If your Windows LLVM install lives in a non-standard location, set `MAESTRO_LLVM_BIN` to the LLVM `bin` directory before running Cargo or Tauri.

### Run

```bash
cd frontend/desktop
npm install
npm run tauri dev
```

### Build

```bash
cd frontend/desktop
npm run tauri build
```

## Tech Stack

- **Desktop Shell**: [Tauri v2](https://tauri.app/) (lightweight, cross-platform)
- **Frontend**: React 19 + TypeScript + Vite + Tailwind CSS v4
- **Backend**: Rust + Tokio async runtime
- **Storage**: File-based (JSON/JSONL) under `~/.maestro/` — no database required

## Local Data

Maestro stores everything in `~/.maestro/`. No hosted backend, no telemetry, no cloud dependency. Your code and context stay on your machine.

## Repo Layout

```
frontend/
  desktop/
    src/            # React UI
    src-tauri/      # Rust backend and orchestration engine
  web/              # Marketing website
```

## Development Checks

```bash
# Desktop TypeScript
cd frontend/desktop && npx tsc --noEmit

# Rust backend
cd frontend/desktop/src-tauri && cargo check
```

## Licence

MIT

---

<div align="center">

**Stop switching tabs. Start orchestrating.**

[maestro.atbas.xyz](https://maestro.atbas.xyz)

</div>
