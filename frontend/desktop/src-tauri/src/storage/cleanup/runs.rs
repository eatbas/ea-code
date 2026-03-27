use std::path::Path;

use chrono::{DateTime, Utc};

use crate::models::{RunFileStatus, RunSummary};

use super::traverse::traverse_runs;

/// Cleans up old runs based on retention policy.
///
/// Iterates all known workspaces via [`traverse_runs`], parses
/// `completed_at` from each run's `summary.json`, and deletes run
/// directories older than `retention_days`.
///
/// Does NOT remove empty session directories — sessions carry metadata
/// that should only be deleted via an explicit `delete_session()` call.
pub fn cleanup_old_runs(retention_days: u32) -> Result<(), String> {
    if retention_days == 0 {
        // Retention disabled
        return Ok(());
    }

    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let mut total_deleted: usize = 0;

    traverse_runs(&mut |run_path, _project_id, _session_id, _run_id| {
        if should_delete_run(run_path, cutoff)? {
            if let Err(e) = std::fs::remove_dir_all(run_path) {
                eprintln!(
                    "Warning: failed to delete old run directory {}: {e}",
                    run_path.display()
                );
            } else {
                total_deleted += 1;
            }
        }
        Ok(())
    })?;

    if total_deleted > 0 {
        println!("Cleaned up {total_deleted} old run(s)");
    }

    Ok(())
}

/// Determines whether a run directory is old enough to be deleted.
fn should_delete_run(run_path: &Path, cutoff: DateTime<Utc>) -> Result<bool, String> {
    let summary_path = run_path.join("summary.json");

    if !summary_path.exists() {
        // No summary.json — fall back to directory modification time.
        let metadata = std::fs::metadata(run_path)
            .map_err(|e| format!("Failed to read directory metadata: {e}"))?;

        if let Ok(modified) = metadata.modified() {
            let modified: DateTime<Utc> = modified.into();
            return Ok(modified < cutoff);
        }
        return Ok(false);
    }

    let contents = std::fs::read_to_string(&summary_path)
        .map_err(|e| format!("Failed to read summary file: {e}"))?;

    let summary: RunSummary = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: failed to parse summary.json: {e}");
            return Ok(false);
        }
    };

    // Only delete completed/failed/cancelled runs.
    let is_terminal = matches!(
        summary.status,
        RunFileStatus::Completed | RunFileStatus::Failed | RunFileStatus::Cancelled
    );

    if !is_terminal {
        return Ok(false);
    }

    // Parse timestamp
    let timestamp_str = summary.completed_at.as_ref().unwrap_or(&summary.started_at);

    let timestamp = match DateTime::parse_from_rfc3339(timestamp_str) {
        Ok(t) => t.with_timezone(&Utc),
        Err(_) => {
            // Fallback: try parsing as Unix milliseconds (legacy format).
            if let Ok(millis) = timestamp_str.parse::<i64>() {
                match DateTime::from_timestamp_millis(millis) {
                    Some(t) => t,
                    None => return Ok(false),
                }
            } else {
                eprintln!("Warning: failed to parse timestamp {timestamp_str}");
                return Ok(false);
            }
        }
    };

    Ok(timestamp < cutoff)
}
