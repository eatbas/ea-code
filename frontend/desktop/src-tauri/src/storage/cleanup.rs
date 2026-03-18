use std::path::Path;

use chrono::{DateTime, Utc};

use crate::models::{RunFileStatus, RunSummary, StorageStats};

use super::{config_dir, remove_run_from_index};

/// Cleans up old runs based on retention policy.
/// Walks projects/*/sessions/*/runs/*, parses completed_at from summary.json,
/// and deletes run directories older than retention_days.
/// H9: Does NOT remove empty session directories - sessions have metadata
/// that should only be deleted via explicit delete_session() call.
pub fn cleanup_old_runs(retention_days: u32) -> Result<(), String> {
    if retention_days == 0 {
        // Retention disabled
        return Ok(());
    }

    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);

    let projects_dir = config_dir()?.join("projects");

    if !projects_dir.exists() {
        return Ok(());
    }

    let mut total_deleted = 0;

    let project_entries = std::fs::read_dir(&projects_dir)
        .map_err(|e| format!("Failed to read projects directory: {e}"))?;

    for project_entry in project_entries {
        let project_entry =
            project_entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let project_path = project_entry.path();

        if !project_path.is_dir() {
            continue;
        }

        let sessions_dir = project_path.join("sessions");
        if !sessions_dir.exists() {
            continue;
        }

        total_deleted += cleanup_sessions_dir(&sessions_dir, cutoff)?;
    }

    if total_deleted > 0 {
        println!("Cleaned up {total_deleted} old run(s)");
    }

    Ok(())
}

/// Scans a sessions directory for old runs and deletes them.
fn cleanup_sessions_dir(sessions_dir: &Path, cutoff: DateTime<Utc>) -> Result<usize, String> {
    let mut deleted = 0;

    let session_entries = std::fs::read_dir(sessions_dir)
        .map_err(|e| format!("Failed to read sessions directory: {e}"))?;

    for session_entry in session_entries {
        let session_entry =
            session_entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let session_path = session_entry.path();

        if !session_path.is_dir() {
            continue;
        }

        let runs_dir = session_path.join("runs");
        if !runs_dir.exists() {
            // H9: Don't delete session even if runs/ doesn't exist
            continue;
        }

        let run_entries = std::fs::read_dir(&runs_dir)
            .map_err(|e| format!("Failed to read runs directory: {e}"))?;

        for run_entry in run_entries {
            let run_entry =
                run_entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
            let run_path = run_entry.path();

            if !run_path.is_dir() {
                continue;
            }

            let should_delete = should_delete_run(&run_path, cutoff)?;

            if should_delete {
                // Remove from index before deleting directory
                if let Some(run_id) = run_path.file_name().and_then(|s| s.to_str()) {
                    if let Err(e) = remove_run_from_index(run_id) {
                        eprintln!("Warning: Failed to remove run {run_id} from index: {e}");
                    }
                }

                if let Err(e) = std::fs::remove_dir_all(&run_path) {
                    eprintln!(
                        "Warning: Failed to delete old run directory {}: {e}",
                        run_path.display()
                    );
                } else {
                    deleted += 1;
                }
            }
        }

        // H9: Do NOT delete session directory even if runs/ is now empty
    }

    Ok(deleted)
}

/// Determines if a run should be deleted based on its age.
fn should_delete_run(run_path: &Path, cutoff: DateTime<Utc>) -> Result<bool, String> {
    let summary_path = run_path.join("summary.json");

    if !summary_path.exists() {
        // No summary.json - check directory modification time as fallback
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
            eprintln!("Warning: Failed to parse summary.json: {e}");
            return Ok(false);
        }
    };

    // Only delete completed/failed/cancelled/crashed runs
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
            // Fallback: try parsing as Unix milliseconds (legacy format)
            if let Ok(millis) = timestamp_str.parse::<i64>() {
                match DateTime::from_timestamp_millis(millis) {
                    Some(t) => t,
                    None => return Ok(false),
                }
            } else {
                eprintln!("Warning: Failed to parse timestamp {timestamp_str}");
                return Ok(false);
            }
        }
    };

    Ok(timestamp < cutoff)
}

/// Gets storage usage statistics.
/// Scans the full projects/*/sessions/*/runs/* hierarchy.
pub fn get_storage_stats() -> Result<StorageStats, String> {
    let mut total_sessions = 0;
    let mut total_runs = 0;
    let mut total_events_bytes = 0u64;

    let projects_dir = config_dir()?.join("projects");

    if projects_dir.exists() {
        let project_entries = std::fs::read_dir(&projects_dir)
            .map_err(|e| format!("Failed to read projects directory: {e}"))?;

        for project_entry in project_entries {
            let project_entry =
                project_entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
            let project_path = project_entry.path();

            if !project_path.is_dir() {
                continue;
            }

            let sessions_dir = project_path.join("sessions");
            if !sessions_dir.exists() {
                continue;
            }

            let session_entries = std::fs::read_dir(&sessions_dir)
                .map_err(|e| format!("Failed to read sessions directory: {e}"))?;

            for session_entry in session_entries {
                let session_entry =
                    session_entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
                let session_path = session_entry.path();

                if !session_path.is_dir() {
                    continue;
                }

                total_sessions += 1;

                let runs_dir = session_path.join("runs");
                if runs_dir.exists() {
                    let run_entries = std::fs::read_dir(&runs_dir)
                        .map_err(|e| format!("Failed to read runs directory: {e}"))?;

                    for run_entry in run_entries {
                        let run_entry = run_entry
                            .map_err(|e| format!("Failed to read directory entry: {e}"))?;
                        let run_path = run_entry.path();

                        if !run_path.is_dir() {
                            continue;
                        }

                        total_runs += 1;

                        // Count events.jsonl size
                        let events_path = run_path.join("events.jsonl");
                        if let Ok(metadata) = std::fs::metadata(&events_path) {
                            total_events_bytes += metadata.len();
                        }
                    }
                }
            }
        }
    }

    Ok(StorageStats {
        total_sessions,
        total_runs,
        total_events_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_stats_default() {
        let stats = StorageStats {
            total_sessions: 0,
            total_runs: 0,
            total_events_bytes: 0,
        };
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_runs, 0);
        assert_eq!(stats.total_events_bytes, 0);
    }
}
