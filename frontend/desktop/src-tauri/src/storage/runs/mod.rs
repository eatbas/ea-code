use crate::models::{RunEvent, RunFileStatus, RunSummary};

use super::{atomic_write, now_rfc3339, validate_id};

pub mod cli_sessions;
pub mod events;
pub mod git;

const SCHEMA_VERSION: u32 = 1;

/// Returns the run directory path under the workspace.
fn run_dir(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<std::path::PathBuf, String> {
    Ok(super::sessions::session_dir(workspace_path, session_id)?
        .join("runs")
        .join(run_id))
}

/// Returns the summary.json file path for a run.
fn summary_path(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(workspace_path, session_id, run_id)?.join("summary.json"))
}

/// Returns the events.jsonl file path for a run.
pub(crate) fn events_path(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(workspace_path, session_id, run_id)?.join("events.jsonl"))
}

/// Creates a new run with initial structure.
/// Creates directories, writes initial summary.json, appends run_start event,
/// and increments the session's run count exactly once.
pub fn create_run(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    prompt: &str,
    max_iterations: u32,
) -> Result<(), String> {
    validate_id(run_id)?;
    validate_id(session_id)?;

    // Create run directory
    let run_dir = run_dir(workspace_path, session_id, run_id)?;
    std::fs::create_dir_all(&run_dir)
        .map_err(|e| format!("Failed to create run directory: {e}"))?;

    // Capture git baseline using workspace path
    let git_baseline = git::capture_git_baseline(workspace_path)?;

    let now = now_rfc3339();
    let summary = RunSummary {
        schema_version: SCHEMA_VERSION,
        id: run_id.to_string(),
        session_id: session_id.to_string(),
        prompt: prompt.to_string(),
        enhanced_prompt: None,
        status: RunFileStatus::Running,
        final_verdict: None,
        current_stage: None,
        current_iteration: None,
        total_iterations: 0,
        max_iterations,
        executive_summary: None,
        files_changed: Vec::new(),
        error: None,
        git_baseline,
        workspace_path: Some(workspace_path.to_string()),
        next_sequence: 1,
        started_at: now.clone(),
        completed_at: None,
    };

    // Write initial summary
    update_summary_internal(workspace_path, session_id, run_id, &summary)?;

    // Create empty events.jsonl file
    let events_file = events_path(workspace_path, session_id, run_id)?;
    atomic_write(&events_file, "")?;

    // Append run_start event (seq=1)
    let run_start = RunEvent::RunStart {
        v: SCHEMA_VERSION,
        seq: 1,
        ts: now,
        prompt: prompt.to_string(),
        max_iterations,
    };
    events::append_event_internal(workspace_path, session_id, run_id, &run_start)?;

    // Update next_sequence to 2 after first event
    let mut summary = summary;
    summary.next_sequence = 2;
    update_summary_internal(workspace_path, session_id, run_id, &summary)?;

    // Increment run count exactly once here
    super::sessions::increment_run_count(workspace_path, session_id)?;

    Ok(())
}

/// Updates the summary.json file atomically.
pub fn update_summary(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    summary: &RunSummary,
) -> Result<(), String> {
    validate_id(run_id)?;
    update_summary_internal(workspace_path, session_id, run_id, summary)
}

pub(crate) fn update_summary_internal(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    summary: &RunSummary,
) -> Result<(), String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = summary_path(workspace_path, session_id, run_id)?;

    let json = serde_json::to_string_pretty(summary)
        .map_err(|e| format!("Failed to serialise summary: {e}"))?;

    atomic_write(&path, &json)
}

/// Reads the summary.json for a run.
pub fn read_summary(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<RunSummary, String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = summary_path(workspace_path, session_id, run_id)?;

    if !path.exists() {
        return Err(format!("Run summary not found: {run_id}"));
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read summary file: {e}"))?;

    let summary: RunSummary = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse summary file: {e}"))?;

    Ok(summary)
}

/// Lists all runs for a session, sorted by started_at descending.
pub fn list_runs(workspace_path: &str, session_id: &str) -> Result<Vec<RunSummary>, String> {
    validate_id(session_id)?;
    let runs_dir = super::sessions::session_dir(workspace_path, session_id)?.join("runs");

    if !runs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();

    let entries =
        std::fs::read_dir(&runs_dir).map_err(|e| format!("Failed to read runs directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_dir() {
            let summary_json = path.join("summary.json");
            if summary_json.exists() {
                match std::fs::read_to_string(&summary_json) {
                    Ok(contents) => match serde_json::from_str::<RunSummary>(&contents) {
                        Ok(summary) => runs.push(summary),
                        Err(e) => eprintln!(
                            "Warning: Failed to parse summary file {}: {e}",
                            summary_json.display()
                        ),
                    },
                    Err(e) => eprintln!(
                        "Warning: Failed to read summary file {}: {e}",
                        summary_json.display()
                    ),
                }
            }
        }
    }

    // Sort by started_at descending (most recent first)
    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Ok(runs)
}

/// Computes files changed for a run using git diff from the baseline.
pub fn compute_files_changed(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<Vec<String>, String> {
    validate_id(run_id)?;
    let summary = read_summary(workspace_path, session_id, run_id)?;

    let Some(baseline) = summary.git_baseline else {
        return Ok(Vec::new());
    };

    let ws = summary.workspace_path.as_deref().unwrap_or(workspace_path);
    git::compute_files_changed_internal(&baseline, ws)
}

// ---- Artifact persistence ----

/// Returns the iteration directory path: .../runs/{rid}/iterations/iter-{N}
fn iteration_dir(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    iteration: u32,
) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(workspace_path, session_id, run_id)?
        .join("iterations")
        .join(format!("iter-{iteration}")))
}

/// Returns the absolute path where an agent should write its output artifact.
///
/// Creates the iteration directory if needed. Returns a `.md` file path.
/// Callers pass this path to the agent prompt so it writes directly there.
pub fn artifact_output_path(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    iteration: u32,
    kind: &str,
) -> Result<std::path::PathBuf, String> {
    validate_id(run_id)?;
    let dir = iteration_dir(workspace_path, session_id, run_id, iteration)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create iteration directory: {e}"))?;
    let filename = format!("{kind}.md");
    Ok(dir.join(filename))
}

/// Returns the workspace-relative path for an artifact (used in agent prompts).
pub fn artifact_relative_path(
    session_id: &str,
    run_id: &str,
    iteration: u32,
    kind: &str,
) -> String {
    format!(".ea-code/sessions/{session_id}/runs/{run_id}/iterations/iter-{iteration}/{kind}.md")
}

/// Writes an artifact file to disk.
///
/// File naming: `iterations/iter-{N}/{kind}.md`
/// Creates the iteration directory if it does not exist.
pub fn write_artifact(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    iteration: u32,
    kind: &str,
    content: &str,
) -> Result<(), String> {
    validate_id(run_id)?;
    let dir = iteration_dir(workspace_path, session_id, run_id, iteration)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create iteration directory: {e}"))?;

    let filename = format!("{kind}.md");
    let path = dir.join(&filename);
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write artifact {filename}: {e}"))?;
    Ok(())
}

/// Reads all artifacts for a run, returning a map of `kind` to `content`.
///
/// Scans `iterations/iter-N/{kind}.md`.
/// If multiple iterations wrote the same kind, the latest iteration wins.
pub fn read_all_artifacts(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<std::collections::HashMap<String, String>, String> {
    validate_id(run_id)?;
    let base = run_dir(workspace_path, session_id, run_id)?;

    let mut artifacts = std::collections::HashMap::new();
    let mut iter_tracker: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    let iters_dir = base.join("iterations");
    if iters_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&iters_dir) {
            for entry in entries.flatten() {
                let dir_path = entry.path();
                if !dir_path.is_dir() {
                    continue;
                }
                let dir_name = match dir_path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let iter_num: u32 =
                    match dir_name.strip_prefix("iter-").and_then(|s| s.parse().ok()) {
                        Some(n) => n,
                        None => continue,
                    };
                if let Ok(files) = std::fs::read_dir(&dir_path) {
                    for file_entry in files.flatten() {
                        let file_path = file_entry.path();
                        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if ext != "md" && ext != "txt" {
                            continue;
                        }
                        if let Some(kind) = file_path.file_stem().and_then(|s| s.to_str()) {
                            let existing_iter = iter_tracker.get(kind).copied().unwrap_or(0);
                            if iter_num >= existing_iter {
                                if let Ok(content) = std::fs::read_to_string(&file_path) {
                                    artifacts.insert(kind.to_string(), content);
                                    iter_tracker.insert(kind.to_string(), iter_num);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(artifacts)
}

/// Re-exports for backward compatibility.
pub use cli_sessions::{read_cli_sessions, update_cli_session, write_cli_sessions};
pub use events::{append_event, next_sequence, read_events};
pub use git::capture_git_baseline;
