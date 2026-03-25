use std::path::Path;

use chrono::{DateTime, Utc};

use crate::models::{RunFileStatus, RunSummary, StorageStats};

use super::config_dir;

/// Cleans up old runs based on retention policy.
/// Iterates all known workspaces, scanning `<workspace>/.ea-code/sessions/*/runs/*`,
/// parses completed_at from summary.json, and deletes run directories older than retention_days.
/// H9: Does NOT remove empty session directories - sessions have metadata
/// that should only be deleted via explicit delete_session() call.
pub fn cleanup_old_runs(retention_days: u32) -> Result<(), String> {
    if retention_days == 0 {
        // Retention disabled
        return Ok(());
    }

    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let mut total_deleted = 0;

    let projects = super::projects::read_projects().unwrap_or_default();

    for project in &projects {
        let ws = std::path::Path::new(&project.path);
        if !ws.exists() {
            continue;
        }

        let sessions_dir = ws.join(".ea-code").join("sessions");
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
/// Iterates all known workspaces, scanning `<workspace>/.ea-code/sessions/*/runs/*`.
pub fn get_storage_stats() -> Result<StorageStats, String> {
    let mut total_sessions = 0;
    let mut total_runs = 0;
    let mut total_events_bytes = 0u64;

    let projects = super::projects::read_projects().unwrap_or_default();

    for project in &projects {
        let ws = std::path::Path::new(&project.path);
        if !ws.exists() {
            continue;
        }

        let sessions_dir = ws.join(".ea-code").join("sessions");
        if !sessions_dir.exists() {
            continue;
        }

        let session_entries = match std::fs::read_dir(&sessions_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for session_entry in session_entries {
            let session_entry = match session_entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let session_path = session_entry.path();

            if !session_path.is_dir() {
                continue;
            }

            total_sessions += 1;

            let runs_dir = session_path.join("runs");
            if runs_dir.exists() {
                let run_entries = match std::fs::read_dir(&runs_dir) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                for run_entry in run_entries {
                    let run_entry = match run_entry {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
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

    Ok(StorageStats {
        total_sessions,
        total_runs,
        total_events_bytes,
    })
}

/// Removes stale temporary files on app startup.
///
/// Safe to run only at startup when no pipeline is active.
pub fn cleanup_stale_temp_files() -> Result<usize, String> {
    let base = config_dir()?;
    let mut cleaned = 0usize;

    // 1. Delete dead .txt prompt files in {config_dir}/prompts/
    let prompts_dir = base.join("prompts");
    if prompts_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&prompts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("txt") {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            eprintln!("Cleanup: removed stale prompt file {}", path.display());
                            cleaned += 1;
                        }
                        Err(e) => {
                            eprintln!(
                                "Cleanup warning: could not remove {}: {e}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
    }

    // 2. Delete dead mcp-config-*.json in config dir root
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("mcp-config-") && name.ends_with(".json") {
                        match std::fs::remove_file(&path) {
                            Ok(()) => {
                                eprintln!(
                                    "Cleanup: removed stale MCP config {}",
                                    path.display()
                                );
                                cleaned += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Cleanup warning: could not remove {}: {e}",
                                    path.display()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Recursively delete orphaned .tmp files anywhere under config dir
    cleaned += remove_tmp_files_recursive(&base);

    // 4. Delete legacy SQLite files matching ea-code.db*
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("ea-code.db") {
                        match std::fs::remove_file(&path) {
                            Ok(()) => {
                                eprintln!(
                                    "Cleanup: removed legacy SQLite file {}",
                                    path.display()
                                );
                                cleaned += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Cleanup warning: could not remove {}: {e}",
                                    path.display()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Delete empty run directories (no summary.json) across all workspaces
    let projects = super::projects::read_projects().unwrap_or_default();
    for project in &projects {
        let ws = std::path::Path::new(&project.path);
        if !ws.exists() {
            continue;
        }
        let sessions_dir = ws.join(".ea-code").join("sessions");
        if !sessions_dir.is_dir() {
            continue;
        }
        if let Ok(session_entries) = std::fs::read_dir(&sessions_dir) {
            for session_entry in session_entries.flatten() {
                let runs_dir = session_entry.path().join("runs");
                if !runs_dir.is_dir() {
                    continue;
                }
                if let Ok(run_entries) = std::fs::read_dir(&runs_dir) {
                    for run_entry in run_entries.flatten() {
                        let run_path = run_entry.path();
                        if !run_path.is_dir() {
                            continue;
                        }
                        let summary = run_path.join("summary.json");
                        if !summary.exists() {
                            match std::fs::remove_dir_all(&run_path) {
                                Ok(()) => {
                                    eprintln!(
                                        "Cleanup: removed empty run dir {}",
                                        run_path.display()
                                    );
                                    cleaned += 1;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Cleanup warning: could not remove {}: {e}",
                                        run_path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if cleaned > 0 {
        eprintln!("Startup cleanup: removed {cleaned} stale file(s)/dir(s)");
    }

    Ok(cleaned)
}

/// Recursively removes `.tmp` files under `dir`, returning the count removed.
fn remove_tmp_files_recursive(dir: &Path) -> usize {
    let mut count = 0;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            count += remove_tmp_files_recursive(&path);
        } else if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("tmp")
        {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    eprintln!("Cleanup: removed orphaned tmp file {}", path.display());
                    count += 1;
                }
                Err(e) => {
                    eprintln!(
                        "Cleanup warning: could not remove {}: {e}",
                        path.display()
                    );
                }
            }
        }
    }
    count
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
