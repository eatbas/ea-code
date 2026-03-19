use crate::models::{RunEvent, RunFileStatus, RunStatus};

use super::sessions;
use super::{config_dir, now_rfc3339};

/// Scans for crashed runs (runs with status "running" but no terminal event).
/// Returns (session_id, run_id) pairs for crashed runs.
pub fn scan_for_crashed_runs() -> Result<Vec<(String, String)>, String> {
    let mut crashed = Vec::new();

    let all_sessions = sessions::list_all_sessions()?;

    for session in all_sessions {
        let runs = super::runs::list_runs(&session.id)?;

        for run in runs {
            // Check if run has status "running" but no active process
            if run.status == RunFileStatus::Running {
                // Read events to check for terminal event
                let events = super::runs::read_events(&run.id)?;

                let has_terminal = events.iter().any(|e| e.is_terminal());

                if !has_terminal {
                    crashed.push((session.id.clone(), run.id.clone()));
                }
            }
        }
    }

    Ok(crashed)
}

/// Recovers a crashed run by:
/// 1. Reading the last event from events.jsonl
/// 2. Updating summary.json with status: "crashed"
/// 3. Appending synthetic run_end event
/// 4. Updating session.json
pub fn recover_run(session_id: &str, run_id: &str) -> Result<(), String> {
    let now = now_rfc3339();

    // Read events to find the last one
    let events = super::runs::read_events(run_id)?;

    if events.is_empty() {
        // Empty events file - mark as crashed with minimal info
        eprintln!("Warning: Empty events file for run {run_id}, creating minimal recovery");
        return recover_empty_run(session_id, run_id, &now);
    }

    let last_event = events.last().unwrap();
    let last_seq = last_event.sequence();
    let last_ts = last_event.timestamp().to_string();

    // Extract current stage and iteration from last event
    let (current_stage, current_iteration) = extract_stage_info(last_event);

    // Read and update summary - use RunStatus::Crashed for consistency
    let mut summary = super::runs::read_summary(run_id)?;
    summary.status = RunFileStatus::Crashed;
    summary.current_stage = current_stage;
    summary.current_iteration = current_iteration;
    summary.error = Some("Run interrupted — app closed or crashed during execution".to_string());
    summary.completed_at = Some(last_ts.clone());

    // Write updated summary
    super::runs::update_summary(run_id, &summary)?;

    // Append synthetic run_end event - status matches summary
    let synthetic_event = RunEvent::RunEnd {
        v: 1,
        seq: last_seq + 1,
        ts: now.clone(),
        status: RunStatus::Crashed,
        verdict: summary
            .final_verdict
            .clone()
            .or(Some(crate::models::JudgeVerdict::NotComplete)),
        error: Some("recovered on startup".to_string()),
        recovered_at: Some(now.clone()),
    };

    // Append the event directly (need session_id)
    append_event_direct(session_id, run_id, &synthetic_event)?;

    // Update session metadata
    sessions::touch_session(
        session_id,
        Some(&summary.prompt),
        Some("crashed"),
        summary.final_verdict.map(|v| format!("{v:?}")).as_deref(),
    )?;

    eprintln!("Recovered crashed run {run_id} in session {session_id}");

    Ok(())
}

/// Extracts stage and iteration info from a run event.
fn extract_stage_info(event: &RunEvent) -> (Option<crate::models::PipelineStage>, Option<u32>) {
    match event {
        RunEvent::StageStart {
            stage, iteration, ..
        } => (Some(stage.clone()), Some(*iteration)),
        RunEvent::StageEnd {
            stage, iteration, ..
        } => (Some(stage.clone()), Some(*iteration)),
        RunEvent::IterationEnd { iteration, .. } => (None, Some(*iteration)),
        _ => (None, None),
    }
}

/// Appends an event directly to a run's events.jsonl.
/// H11: Flushes after write for durability.
fn append_event_direct(session_id: &str, run_id: &str, event: &RunEvent) -> Result<(), String> {
    use std::io::Write;

    let path = sessions::session_dir(session_id)?
        .join("runs")
        .join(run_id)
        .join("events.jsonl");

    let line =
        serde_json::to_string(event).map_err(|e| format!("Failed to serialise event: {e}"))?;

    // H11: Append to file with explicit flush for durability
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open events file: {e}"))?;

    writeln!(file, "{line}").map_err(|e| format!("Failed to append event: {e}"))?;

    // H11: Flush to ensure data is written to OS buffers
    file.flush()
        .map_err(|e| format!("Failed to flush events file: {e}"))?;

    Ok(())
}

/// Recovers a run with empty events file.
/// Creates minimal summary and synthetic event to allow cleanup.
fn recover_empty_run(session_id: &str, run_id: &str, now: &str) -> Result<(), String> {
    // Try to read existing summary or create minimal one
    let mut summary = match super::runs::read_summary(run_id) {
        Ok(s) => s,
        Err(_) => {
            return Err(format!(
                "Cannot recover run {run_id}: no summary and no events"
            ));
        }
    };

    summary.status = RunFileStatus::Crashed;
    summary.error = Some("Run interrupted — no events recorded".to_string());
    summary.completed_at = Some(now.to_string());

    super::runs::update_summary(run_id, &summary)?;

    // Update session metadata
    sessions::touch_session(session_id, Some(&summary.prompt), Some("crashed"), None)?;

    eprintln!("Recovered empty run {run_id} in session {session_id}");

    Ok(())
}

/// Runs crash recovery on all crashed runs found.
/// Returns the number of successfully recovered runs.
/// H10: Passes session_id directly from scan results - no redundant lookup needed.
pub fn recover_all_crashed() -> Result<usize, String> {
    let crashed = scan_for_crashed_runs()?;
    let mut success_count = 0;

    // H10: Pass session_id directly from scan results - no redundant lookup needed
    for (session_id, run_id) in crashed {
        match recover_run(&session_id, &run_id) {
            Ok(()) => success_count += 1,
            Err(e) => eprintln!("Failed to recover run {run_id}: {e}"),
        }
    }

    Ok(success_count)
}

/// Migrates data from the old config directory (`AppData/Roaming/ea-code/`) to `~/.ea-code/`.
/// Only acts when old exists and new does not.
pub fn migrate_config_dir() -> Result<(), String> {
    let old_dir = dirs::config_dir()
        .ok_or_else(|| "Unable to determine old config directory".to_string())?
        .join("ea-code");
    let new_dir = config_dir()?;

    if old_dir.exists() && !new_dir.exists() {
        eprintln!(
            "Migrating config from {} to {}",
            old_dir.display(),
            new_dir.display()
        );
        if let Some(parent) = new_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent for new config dir: {e}"))?;
        }
        std::fs::rename(&old_dir, &new_dir).map_err(|e| {
            format!(
                "Failed to migrate config: {e}. Move {} to {} manually.",
                old_dir.display(),
                new_dir.display()
            )
        })?;
    }
    Ok(())
}
