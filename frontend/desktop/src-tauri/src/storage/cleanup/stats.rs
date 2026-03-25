use crate::models::StorageStats;

/// Gets storage usage statistics.
/// Iterates all known workspaces, scanning `<workspace>/.ea-code/sessions/*/runs/*`.
pub fn get_storage_stats() -> Result<StorageStats, String> {
    let mut total_sessions = 0;
    let mut total_runs = 0;
    let mut total_events_bytes = 0u64;

    let projects = crate::storage::projects::read_projects().unwrap_or_default();

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
