//! Automatic retention cleanup for old pipeline runs.

use diesel::prelude::*;

use crate::schema::runs;

use super::DbPool;

/// Deletes completed runs whose `completed_at` is older than `retention_days`.
/// Returns the number of runs deleted. Cascade foreign keys ensure child rows
/// (iterations, stages, artefacts, questions) are removed automatically.
pub fn cleanup_old_runs(pool: &DbPool, retention_days: i32) -> Result<usize, String> {
    if retention_days <= 0 {
        return Ok(0);
    }

    let mut conn = super::get_conn(pool)?;

    let cutoff = chrono::Utc::now()
        - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let deleted = diesel::delete(
        runs::table
            .filter(runs::status.eq("completed").or(runs::status.eq("cancelled").or(runs::status.eq("failed"))))
            .filter(runs::completed_at.is_not_null())
            .filter(runs::completed_at.lt(&cutoff_str)),
    )
    .execute(&mut conn)
    .map_err(|e| format!("Failed to cleanup old runs: {e}"))?;

    if deleted > 0 {
        eprintln!("[cleanup] Deleted {deleted} runs older than {retention_days} days");
    }

    Ok(deleted)
}

/// Runs SQLite VACUUM to reclaim disk space after deletions.
/// **Note:** This rewrites the entire database file and holds an exclusive lock.
/// Prefer [`pragma_optimize`] for routine maintenance; reserve VACUUM for manual
/// compaction via the DB tools UI.
pub fn vacuum(pool: &DbPool) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::sql_query("VACUUM;")
        .execute(&mut conn)
        .map_err(|e| format!("VACUUM failed: {e}"))?;

    Ok(())
}

/// Lightweight maintenance: lets SQLite refresh its query-planner statistics.
/// Safe to call on every startup — takes milliseconds even on large databases.
pub fn pragma_optimize(pool: &DbPool) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::sql_query("PRAGMA optimize;")
        .execute(&mut conn)
        .map_err(|e| format!("PRAGMA optimize failed: {e}"))?;

    Ok(())
}
