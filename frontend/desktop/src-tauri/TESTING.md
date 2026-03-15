# Migration Testing Guide

This document provides a comprehensive testing checklist for verifying the SQLite to JSONL migration.

---

## Pre-Migration Checklist

Before running the new version for the first time:

- [ ] Backup existing `~/.config/ea-code/` directory:
  ```bash
  cp -r ~/.config/ea-code ~/.config/ea-code.backup.$(date +%Y%m%d)
  ```
- [ ] Note existing settings (CLI paths, model selections)
- [ ] Note existing sessions and their run counts
- [ ] Ensure the old `ea-code.db` file exists (for migration verification)

---

## Post-Migration Verification

### Application Startup

- [ ] App launches without errors
- [ ] No panic messages in console
- [ ] Settings are preserved (check CLI paths, model selections)
- [ ] Legacy `ea-code.db` is preserved as backup (not deleted)

### Settings Migration

- [ ] Launch app with existing SQLite DB present
- [ ] Verify `settings.json` is created in `~/.config/ea-code/`
- [ ] Verify settings values match previous configuration
- [ ] Verify `retention_days` defaults to 90 if not previously set

### Session Management

- [ ] Create new session from project view
- [ ] List sessions in sidebar (verify they appear)
- [ ] Open session detail (verify runs load)
- [ ] Delete session (verify it's removed from disk)

### Run Execution

- [ ] Execute a pipeline run
- [ ] Verify `events.jsonl` is created in `sessions/<id>/runs/<rid>/`
- [ ] Verify `summary.json` is created alongside events.jsonl
- [ ] Check `session.json` is updated with:
  - [ ] `lastPrompt` - the submitted prompt
  - [ ] `lastStatus` - run status (running, completed, failed, etc.)
  - [ ] `runCount` - incremented by 1

### Event Log Verification

For a completed run, verify `events.jsonl` contains:

- [ ] `run_start` event (seq=1) with prompt and max_iterations
- [ ] `stage_start` events for each executed stage
- [ ] `stage_end` events with duration and status
- [ ] `iteration_end` events after judge stages
- [ ] `run_end` terminal event with final status

### Crash Recovery

- [ ] Start a pipeline run
- [ ] Kill the app during execution (simulate crash):
  - Windows: Task Manager → End Task
  - macOS/Linux: `kill -9 <pid>`
- [ ] Restart the app
- [ ] Verify the run is marked as `crashed` in the UI
- [ ] Check that a synthetic `run_end` event was appended to events.jsonl
- [ ] Verify session.json shows `lastStatus: "crashed"`

### History View

- [ ] Load session list for a project
- [ ] Verify sessions sorted by `updated_at` descending
- [ ] Load run detail with events timeline
- [ ] Verify events render correctly in timeline
- [ ] Verify no console errors about missing events

### Retention Cleanup

- [ ] Set `retention_days` to 1 in settings
- [ ] Create a test session with a run
- [ ] Modify the run's `completed_at` timestamp to be >1 day old:
  ```bash
  # Edit sessions/<id>/runs/<rid>/summary.json
  # Change completed_at to an old date
  ```
- [ ] Restart app (triggers cleanup)
- [ ] Verify old run directory is deleted
- [ ] Verify empty session directory is removed

### Pause/Resume with Question

- [ ] Start a run that will ask a question (e.g., plan approval)
- [ ] When question appears, verify `question` event in events.jsonl
- [ ] Answer the question
- [ ] Verify answer appears in events.jsonl
- [ ] Verify run continues correctly

### File Structure Verification

Verify the following directory structure exists:

```
~/.config/ea-code/
├── settings.json           # App settings
├── skills/                 # Skill definitions
│   ├── <skill-id>.json
│   └── ...
├── sessions/               # Session storage
│   └── <session-id>/
│       ├── session.json    # Session metadata
│       └── runs/
│           └── <run-id>/
│               ├── summary.json   # Run summary
│               └── events.jsonl   # Event log
└── mcp.json               # MCP server config
```

### Storage Stats

- [ ] Open Settings → Storage (or equivalent)
- [ ] Verify storage stats show:
  - [ ] Correct session count
  - [ ] Correct run count
  - [ ] Total events bytes
  - [ ] Storage path

---

## Manual Verification Commands

### Check Settings

```bash
cat ~/.config/ea-code/settings.json | jq .
```

### Check Session Metadata

```bash
cat ~/.config/ea-code/sessions/<session-id>/session.json | jq .
```

### Check Run Summary

```bash
cat ~/.config/ea-code/sessions/<session-id>/runs/<run-id>/summary.json | jq .
```

### Check Event Log

```bash
# View all events
cat ~/.config/ea-code/sessions/<session-id>/runs/<run-id>/events.jsonl | jq .

# Count events
wc -l ~/.config/ea-code/sessions/<session-id>/runs/<run-id>/events.jsonl

# Check for terminal event
grep "run_end" ~/.config/ea-code/sessions/<session-id>/runs/<run-id>/events.jsonl
```

### List All Sessions

```bash
ls -la ~/.config/ea-code/sessions/
```

### Calculate Storage Size

```bash
du -sh ~/.config/ea-code/
du -sh ~/.config/ea-code/sessions/*/
```

---

## Performance Benchmarks

Compare before/after migration:

| Metric | SQLite | JSONL | Notes |
|--------|--------|-------|-------|
| App startup time | | | Should be similar |
| Session list load | | | Should be faster (no DB query) |
| Run detail load | | | Should be faster (file read vs query) |
| Event timeline scroll | | | Should be smoother |
| Storage size | | | May be larger (text vs binary) |

---

## Troubleshooting

### Settings Not Migrated

1. Check if `~/.config/ea-code/ea-code.db` exists
2. Check console for migration errors
3. Manually copy settings from old DB if needed

### Sessions Not Appearing

1. Verify `~/.config/ea-code/sessions/` exists
2. Check each session has `session.json`
3. Check for JSON parsing errors in console

### Events Not Loading

1. Verify `events.jsonl` exists in run directory
2. Check file is valid JSONL (one JSON object per line)
3. Check for malformed events in the file

### Crash Recovery Not Working

1. Verify run has status "running" in summary.json
2. Check that no `run_end` event exists in events.jsonl
3. Check console for recovery messages on startup

---

## Regression Testing

Verify these existing features still work:

- [ ] Project/workspace selection
- [ ] CLI path configuration
- [ ] Model selection
- [ ] Skill management (create, edit, delete)
- [ ] MCP server configuration
- [ ] Pipeline cancellation
- [ ] Pipeline pause/resume
- [ ] Theme switching
- [ ] Auto-updater check
