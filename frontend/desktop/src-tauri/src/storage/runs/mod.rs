use crate::models::{RunEvent, RunFileStatus, RunSummary};

use super::{add_run_to_index, atomic_write, get_session_for_run, now_rfc3339, validate_id};

pub mod events;
pub mod git;

const SCHEMA_VERSION: u32 = 1;

/// Returns the run directory path via the project hierarchy.
fn run_dir(session_id: &str, run_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(super::sessions::session_dir(session_id)?
        .join("runs")
        .join(run_id))
}

/// Returns the summary.json file path for a run.
fn summary_path(session_id: &str, run_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(session_id, run_id)?.join("summary.json"))
}

/// Returns the events.jsonl file path for a run.
pub(crate) fn events_path(session_id: &str, run_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(session_id, run_id)?.join("events.jsonl"))
}

/// Creates a new run with initial structure.
/// Creates directories, writes initial summary.json, appends run_start event,
/// and increments the session's run count exactly once.
pub fn create_run(
    run_id: &str,
    session_id: &str,
    prompt: &str,
    max_iterations: u32,
    workspace_path: &str,
) -> Result<(), String> {
    validate_id(run_id)?;
    validate_id(session_id)?;

    // Create run directory
    let run_dir = run_dir(session_id, run_id)?;
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
    update_summary_internal(session_id, run_id, &summary)?;

    // Create empty events.jsonl file
    let events_file = events_path(session_id, run_id)?;
    atomic_write(&events_file, "")?;

    // Append run_start event (seq=1)
    let run_start = RunEvent::RunStart {
        v: SCHEMA_VERSION,
        seq: 1,
        ts: now,
        prompt: prompt.to_string(),
        max_iterations,
    };
    events::append_event_internal(session_id, run_id, &run_start)?;

    // Update next_sequence to 2 after first event
    let mut summary = summary;
    summary.next_sequence = 2;
    update_summary_internal(session_id, run_id, &summary)?;

    // Increment run count exactly once here
    super::sessions::increment_run_count(session_id)?;

    // Add run to index for O(1) lookups
    add_run_to_index(run_id, session_id)?;

    Ok(())
}

/// Updates the summary.json file atomically.
pub fn update_summary(run_id: &str, summary: &RunSummary) -> Result<(), String> {
    validate_id(run_id)?;
    let session_id = get_session_for_run(run_id)?;
    update_summary_internal(&session_id, run_id, summary)
}

pub(crate) fn update_summary_internal(
    session_id: &str,
    run_id: &str,
    summary: &RunSummary,
) -> Result<(), String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = summary_path(session_id, run_id)?;

    let json = serde_json::to_string_pretty(summary)
        .map_err(|e| format!("Failed to serialise summary: {e}"))?;

    atomic_write(&path, &json)
}

/// Reads the summary.json for a run.
pub fn read_summary(run_id: &str) -> Result<RunSummary, String> {
    validate_id(run_id)?;
    let session_id = get_session_for_run(run_id)?;
    read_summary_internal(&session_id, run_id)
}

pub(crate) fn read_summary_internal(session_id: &str, run_id: &str) -> Result<RunSummary, String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = summary_path(session_id, run_id)?;

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
///
/// Note: Sorting relies on RFC 3339 timestamp format (e.g., "2026-03-11T14:30:00Z").
/// This format sorts lexicographically for timestamps in the same timezone.
pub fn list_runs(session_id: &str) -> Result<Vec<RunSummary>, String> {
    validate_id(session_id)?;
    let runs_dir = super::sessions::session_dir(session_id)?.join("runs");

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
    // Note: RFC 3339 timestamps sort lexicographically when in the same timezone
    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Ok(runs)
}

/// Computes files changed for a run using git diff from the baseline.
pub fn compute_files_changed(run_id: &str) -> Result<Vec<String>, String> {
    validate_id(run_id)?;
    let session_id = get_session_for_run(run_id)?;
    let summary = read_summary_internal(&session_id, run_id)?;

    let Some(baseline) = summary.git_baseline else {
        return Ok(Vec::new());
    };

    let workspace_path = summary
        .workspace_path
        .or_else(|| {
            super::sessions::read_session(&summary.session_id)
                .ok()
                .map(|s| s.project_path)
        })
        .ok_or_else(|| "No workspace path found for run".to_string())?;

    git::compute_files_changed_internal(&baseline, &workspace_path)
}

// ---- Artifact persistence ----

/// Returns the artifacts directory path for a run.
fn artifacts_dir(session_id: &str, run_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(session_id, run_id)?.join("artifacts"))
}

/// Returns the absolute path where an agent should write its output artifact.
///
/// Creates the artifacts directory if needed. Returns a `.md` file path.
/// Callers pass this path to the agent prompt so it writes directly there.
pub fn artifact_output_path(
    run_id: &str,
    iteration: u32,
    kind: &str,
) -> Result<std::path::PathBuf, String> {
    validate_id(run_id)?;
    let session_id = get_session_for_run(run_id)?;
    let dir = artifacts_dir(&session_id, run_id)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create artifacts directory: {e}"))?;
    let filename = format!("{iteration}_{kind}.md");
    Ok(dir.join(filename))
}

/// Writes an artifact file to disk.
///
/// File naming: `{iteration}_{kind}.md`
/// Creates the artifacts directory if it does not exist.
pub fn write_artifact(
    run_id: &str,
    iteration: u32,
    kind: &str,
    content: &str,
) -> Result<(), String> {
    validate_id(run_id)?;
    let session_id = get_session_for_run(run_id)?;
    let dir = artifacts_dir(&session_id, run_id)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create artifacts directory: {e}"))?;

    let filename = format!("{iteration}_{kind}.md");
    let path = dir.join(&filename);
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write artifact {filename}: {e}"))?;
    Ok(())
}

/// Reads all artifacts for a run, returning a map of `kind` to `content`.
///
/// If multiple iterations wrote the same kind, the latest iteration wins.
pub fn read_all_artifacts(
    run_id: &str,
) -> Result<std::collections::HashMap<String, String>, String> {
    validate_id(run_id)?;
    let session_id = get_session_for_run(run_id)?;
    let dir = artifacts_dir(&session_id, run_id)?;

    let mut artifacts = std::collections::HashMap::new();
    let mut iter_tracker: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    if !dir.exists() {
        return Ok(artifacts);
    }

    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("Failed to read artifacts directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read artifact entry: {e}"))?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "txt" {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Some(underscore_pos) = stem.find('_') {
                let iter_num: u32 = stem[..underscore_pos].parse().unwrap_or(0);
                let kind = &stem[underscore_pos + 1..];
                if kind.is_empty() {
                    continue;
                }
                let existing_iter = iter_tracker.get(kind).copied().unwrap_or(0);
                if iter_num >= existing_iter {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            artifacts.insert(kind.to_string(), content);
                            iter_tracker.insert(kind.to_string(), iter_num);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to read artifact {}: {e}", path.display());
                        }
                    }
                }
            }
        }
    }

    Ok(artifacts)
}

/// Re-exports for backward compatibility.
pub use events::{append_event, next_sequence, read_events};
pub use git::capture_git_baseline;
