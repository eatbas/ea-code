use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::logs;

use super::models::{LogRow, NewLog};

/// Inserts a single log line. Intended to be called fire-and-forget.
pub fn insert(pool: &DbPool, run_id: &str, stage: &str, line: &str, stream: &str) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    diesel::insert_into(logs::table)
        .values(&NewLog {
            run_id,
            stage,
            line,
            stream,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert log: {e}"))?;

    Ok(())
}

/// Returns logs for a given run with offset/limit pagination.
pub fn get_for_run(
    pool: &DbPool,
    run_id: &str,
    offset: i64,
    limit: i64,
) -> Result<Vec<LogRow>, String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    logs::table
        .filter(logs::run_id.eq(run_id))
        .order(logs::created_at.asc())
        .offset(offset)
        .limit(limit)
        .load::<LogRow>(&mut conn)
        .map_err(|e| format!("Failed to get logs: {e}"))
}
