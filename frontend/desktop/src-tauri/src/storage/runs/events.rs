use crate::models::RunEvent;

use super::{events_path, validate_id};

/// Appends a single event to the events.jsonl file.
pub fn append_event(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    event: RunEvent,
) -> Result<(), String> {
    validate_id(run_id)?;
    append_event_internal(workspace_path, session_id, run_id, &event)
}

pub(crate) fn append_event_internal(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    event: &RunEvent,
) -> Result<(), String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = events_path(workspace_path, session_id, run_id)?;

    let line =
        serde_json::to_string(event).map_err(|e| format!("Failed to serialise event: {e}"))?;

    // H11: Append to file with explicit flush for durability
    use std::io::Write;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create events directory: {e}"))?;
    }

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

/// Reads all events from events.jsonl for a run.
pub fn read_events(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<Vec<RunEvent>, String> {
    validate_id(run_id)?;
    read_events_internal(workspace_path, session_id, run_id)
}

pub(crate) fn read_events_internal(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<Vec<RunEvent>, String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = events_path(workspace_path, session_id, run_id)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read events file: {e}"))?;

    let mut events = Vec::new();
    for (line_num, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<RunEvent>(line) {
            Ok(event) => events.push(event),
            Err(e) => {
                eprintln!(
                    "Warning: Skipping malformed event at line {} in {}: {e}",
                    line_num + 1,
                    path.display()
                );
                continue;
            }
        }
    }

    Ok(events)
}

/// Gets the next sequence number for events in a run.
/// Calculates from the line count of events.jsonl for O(1) amortised cost.
pub fn next_sequence(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<u64, String> {
    validate_id(run_id)?;
    next_sequence_internal(workspace_path, session_id, run_id)
}

/// Calculates next sequence from file line count.
pub(crate) fn next_sequence_internal(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<u64, String> {
    validate_id(session_id)?;
    validate_id(run_id)?;
    let path = events_path(workspace_path, session_id, run_id)?;

    if !path.exists() {
        return Ok(1);
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read events file: {e}"))?;

    let count = contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count() as u64;

    Ok(count + 1)
}
