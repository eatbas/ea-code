# Review of `opus-sqlite-2-jsonl-orcflow-update.md`

## Summary

The current plan has one strong idea and one risky assumption.

- The strong idea is to stop storing bulky run artefacts and raw stage output in SQLite.
- The risky assumption is that this should become a full replacement of SQLite for all persistence.

I think the plan should be narrowed for the first implementation. JSONL is a good fit for append-only run history. It is not automatically the best fit for all app metadata.

---

## What Is Good in the Current Plan

- It correctly identifies that the pipeline already communicates in memory and does not need the database as a live hand-off layer.
- It correctly challenges the value of storing large raw agent outputs and git diffs long term.
- It moves towards a more debuggable and human-readable persistence model for run history.
- It simplifies the Judge prompt by relying more on the repository state instead of duplicating context.

---

## What Is Wrong or Too Risky

### 1. The scope is too large

The plan mixes two separate changes:

- reducing run-history storage complexity
- replacing SQLite everywhere

These should not be bundled together. The real pain described in the plan is mostly about run artefacts, iteration detail, and raw stage output. That does not justify rewriting settings, projects, sessions, skills, and MCP persistence in the same change.

### 2. Persisted live run state is being removed too aggressively

The proposal removes `current_stage`, `current_iteration`, and `current_stage_started_at` as persisted state and says live state should exist only via Tauri events.

That is fragile. If the app reloads or reconnects during a run, event-only state disappears. The current UI uses persisted live state for active runs, timing, and control surfaces. Removing it entirely risks a visible regression.

### 3. The proposed file model is too thin for the current history UX

The plan says `session.json` should only contain:

- `title`
- `projectPath`
- `createdAt`
- `updatedAt`

That is not enough for efficient session listing. The current product shape also wants:

- run count
- last prompt
- last status
- recent activity ordering

Without a compact summary layer, the app will need to scan run logs just to render the sidebar and session list.

### 4. `git diff --name-only <start-ref>..HEAD` is not a reliable run boundary

This is not safe when:

- the workspace is dirty before the run starts
- the user edits files during the run
- the branch moves independently

The plan needs a real run-start baseline, not just a ref comparison at completion.

### 5. The JSONL event format is under-specified

The sample format is fine as a sketch, but not yet durable enough as a storage contract.

Missing pieces include:

- schema version
- event timestamp on every line
- monotonic event sequence number
- explicit terminal events for `completed`, `failed`, `cancelled`
- crash-recovery rules
- partial-write and corrupted-line handling
- clear start/end event naming for each stage

### 6. Judge simplification may remove useful signal

I agree that Judge should inspect git directly. I do not agree that all reviewer and fixer hand-off context should necessarily disappear.

Reviewer output often contains:

- unresolved blockers
- test gaps
- explicit failure conditions
- reasoning that is not visible from the diff alone

The better simplification is to remove raw prompt stuffing, not all structured hand-off.

### 7. Retention logic is incomplete

The plan says cleanup should parse the `complete` line timestamp and delete old JSONL files. That fails for:

- cancelled runs
- failed runs
- crashed runs with no terminal line
- partially written files

Cleanup needs rules for incomplete and malformed run logs.

### 8. The migration sequence is too big-bang

The current implementation plan removes Diesel and the `db/` module too early.

There is no safe intermediate state such as:

- dual-read support
- importer validation
- fallback to old data
- one release cycle with migration telemetry

That creates unnecessary delivery risk.

---

## What We Should Alter

### 1. Narrow the first migration

For the first implementation:

- keep SQLite for settings, projects, sessions, skills, and MCP
- move only run history and bulky run artefacts to file storage

This captures most of the value with much less risk.

### 2. Use a per-run directory, not only a flat JSONL file

Recommended structure:

```text
~/.config/ea-code/
  sessions/
    <session-id>/
      session.json
      runs/
        <run-id>/
          summary.json
          events.jsonl
```

Why this is better:

- `summary.json` gives fast reads for history views
- `events.jsonl` remains append-only and debuggable
- partial or failed runs are easier to recover
- future extensions do not force repeated JSONL scans

### 3. Keep a minimal persisted live snapshot

Even if full live state no longer lives in SQLite, the system should still persist minimal active-run state somewhere durable:

- current stage
- current iteration
- run status
- stage start time

That keeps reload/restart behaviour intact.

### 4. Introduce a formal schema before implementation

Add a design phase for the file models:

- Rust serde structs
- TypeScript equivalents
- versioning strategy
- event ordering rules
- terminal-state guarantees
- recovery rules for incomplete logs

### 5. Record a proper run baseline

At run start, capture enough baseline information to determine what changed because of the run. At minimum, this should not rely only on `HEAD..HEAD` style comparison after the fact.

### 6. Replace raw reviewer/fixer hand-off with compact structured data

Do not pass large raw outputs if they are not needed. Instead, consider a compact summary such as:

- unresolved blockers
- warnings
- tests run or not run
- fixer acknowledgement

That gives Judge useful signal without preserving full transcript bloat.

### 7. Add corruption and crash handling rules

The storage layer should define how it handles:

- truncated final lines
- malformed JSONL records
- runs with no terminal event
- app crash during write

This should be part of the storage contract, not an afterthought.

### 8. Do not remove Diesel until the replacement has proved itself

The final removal of SQLite should happen only after:

- import works
- reads work
- history views work
- retention works
- active-run recovery works

The plan should explicitly treat dependency removal as the last step, not the centre of the rewrite.

---

## Recommended Revised Plan

## Phase 0: Define the storage contract

1. Define Rust and TypeScript models for:
   - `RunSummary`
   - `RunEvent`
   - `SessionSnapshot`
   - active run snapshot
2. Add schema version fields.
3. Define terminal event rules for `completed`, `failed`, `cancelled`.
4. Define malformed-line and crash-recovery behaviour.

## Phase 1: Move run history only

1. Keep SQLite for settings, projects, sessions, skills, and MCP.
2. Add file-backed run storage under the session directory.
3. Write `summary.json` plus `events.jsonl` for each run.
4. Stop storing large raw outputs and diff artefacts in SQLite.
5. Keep only compact run metadata in SQLite if the current UI still needs it during transition.

## Phase 2: Adapt the orchestrator

1. Replace run, iteration, stage, question, and artefact persistence with file-backed writes.
2. Keep a minimal persisted active-run snapshot.
3. Remove diff capture as a stored artefact.
4. Simplify Judge inputs, but keep compact structured review/fix findings if needed.
5. Capture changed files from a real run-start baseline.

## Phase 3: Adapt history reads

1. Update session memory to read run summaries from file storage.
2. Update history commands to read run details from `summary.json` and `events.jsonl`.
3. Keep session listing fast by using session metadata rather than scanning every run log on every read.

## Phase 4: Migration and rollout

1. Add a one-time importer for old run history from SQLite into the new file layout.
2. Leave the old database in place as fallback.
3. Support reading imported file-backed runs before removing old code.
4. Verify existing user histories are preserved.

## Phase 5: Optional second migration

Only after the run-history migration is stable, decide whether settings, projects, skills, MCP, and sessions should also leave SQLite.

This should be a separate decision with separate trade-offs, not assumed up front.

## Phase 6: Remove old storage code

1. Remove old run-history tables and code paths once file storage is stable.
2. Remove Diesel entirely only if all remaining app metadata has also been intentionally migrated.

---

## Decision I Recommend

I do recommend:

- moving run history to files
- removing bulky raw stage output storage
- removing diff artefact persistence
- simplifying the Judge input

I do not recommend, in the same change:

- deleting SQLite wholesale
- making live run state event-only
- relying on JSONL scanning alone for all history reads
- removing all reviewer/fixer signal from Judge context

---

## Short Version

The current plan is best reframed as:

> "Move run history from SQLite to a file-backed summary-plus-event-log model, while keeping SQLite for app metadata until the new storage proves itself."

That version is safer, easier to validate, and more closely aligned with the actual problem being solved.
