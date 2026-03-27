use crate::models::StorageStats;

use super::traverse::traverse_sessions_and_runs;

/// Gets storage usage statistics.
///
/// Iterates all known workspaces via [`traverse_sessions_and_runs`],
/// counting sessions, runs, and total `events.jsonl` size.
pub fn get_storage_stats() -> Result<StorageStats, String> {
    let mut total_sessions: usize = 0;
    let mut total_runs: usize = 0;
    let mut total_events_bytes: u64 = 0;

    traverse_sessions_and_runs(
        &mut |_session_path, _project_id, _session_id| {
            total_sessions += 1;
            Ok(())
        },
        &mut |run_path, _project_id, _session_id, _run_id| {
            total_runs += 1;

            // Count events.jsonl size
            let events_path = run_path.join("events.jsonl");
            if let Ok(metadata) = std::fs::metadata(&events_path) {
                total_events_bytes += metadata.len();
            }

            Ok(())
        },
    )?;

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
