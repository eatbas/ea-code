<div align="center">

# Maestro

### Stop Paying for Five AI Subscriptions and Only Using One

You're subscribed to Claude, Gemini, Codex, Kimi, and OpenCode. But right now, you're copy-pasting between them, switching tabs, and hoping one model gets it right on the first try.

**What if they all worked together instead?**

Maestro is a desktop app that wires your AI coding CLIs into a single orchestrated pipeline — where each agent plays the role it's best at, and the result is better than anything one model could produce alone.

[Website](https://maestro.atbas.xyz) &#8226; [Get Started](#getting-started) &#8226; [How It Works](#how-the-pipeline-works)

</div>

---

## The Problem

You're paying for multiple AI coding subscriptions. You use Claude for planning, Gemini for quick questions, Codex for generation. But you're doing the orchestration manually — switching between tools, re-explaining context, and judging the output yourself.

That's expensive, slow, and exhausting.

## The Solution

Maestro puts every AI CLI you already pay for into a structured pipeline:

- **One agent sharpens your prompt** so the task is crystal clear
- **Up to three agents plan in parallel** — you get multiple perspectives, not one guess
- **An auditor pressure-tests the plan** before a single line is written
- **A coder implements it**, then **up to three reviewers critique it independently**
- **A fixer applies the feedback**, and **a judge decides if the task is done**
- **If not? It loops automatically** — refining, regenerating, and reviewing until the job is truly complete

You assign the roles. Claude plans, Codex codes, Gemini reviews — or any combination you want. Each model does what it's best at. The result is code that's been planned, written, reviewed, and approved by multiple AI agents working together.

## Supported AI Backends

| Backend | CLI |
|---------|-----|
| Claude | Claude Code |
| Codex | GitHub Copilot CLI |
| Gemini | Google Gemini CLI |
| Kimi | Kimi CLI |
| OpenCode | OpenCode CLI |

Use one, use all five — assign any backend to any stage.

## How The Pipeline Works

```
 Your Prompt
     |
     v
 [1. Prompt Enhance] -----> Clarifies and sharpens the task
     |
     v
 [2. Skill Select] -------> Pulls in relevant local guidance
     |
     v
 [3. Plan x3] ------------> Up to 3 agents draft plans IN PARALLEL
     |
     v
 [4. Plan Audit] ----------> Pressure-tests the chosen plan
     |
     v
 [5. Code] ----------------> Implements the change
     |
     v
 [6. Review x3] -----------> Up to 3 agents review IN PARALLEL
     |
     v
 [7. Review Merge] --------> Combines all reviewer feedback
     |
     v
 [8. Fix] -----------------> Applies required changes
     |
     v
 [9. Judge] -----+---------> COMPLETE? Ship it.
                  |
                  +---------> NOT COMPLETE? Loop back with full context.
     |
     v
 [10. Executive Summary] --> Records what happened
```

The judge isn't the same agent that wrote the code. That's the point — no model marks its own homework.

## Why This Beats Using One Agent

| Single Agent | Maestro |
|---|---|
| One model plans, codes, reviews, and judges its own work | Different agents specialise in each role |
| Blind spots go unnoticed | Parallel reviewers catch what one misses |
| You manually re-prompt when output is wrong | Auto-loops with full context until the judge approves |
| Context gets lost between sessions | Session memory carries continuity across runs |
| You pay for 5 subscriptions and use 1 at a time | Every subscription earns its keep |

## Desktop App Highlights

- **Project Picker** — switch between repositories instantly
- **Session History** — every task grouped in its own thread with full traceability
- **Live Run Timeline** — watch stages execute with real-time logs, diffs, and artefacts
- **Agent Assignment** — configure which backend handles which stage
- **Plan Approval Gates** — pause before execution to review, revise, or reject
- **Pause / Resume / Cancel** — full control during runs
- **Skill Editor** — create reusable instructions for domain-specific guidance
- **MCP Integrations** — connect Model Context Protocol servers for external tools and context
- **CLI Health Checks** — verify agent availability and update CLIs in-app
- **Fully Local** — everything stored on your machine, no cloud backend required

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
