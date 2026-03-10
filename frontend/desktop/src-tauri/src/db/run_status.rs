use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::runs;

fn update_run_status(
    pool: &DbPool,
    run_id: &str,
    from_statuses: &[&str],
    next_status: &str,
    completed_at: Option<&str>,
    clear_stage: bool,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    if clear_stage {
        diesel::update(
            runs::table
                .filter(runs::id.eq(run_id))
                .filter(runs::status.eq_any(from_statuses)),
        )
        .set((
            runs::status.eq(next_status),
            runs::completed_at.eq(completed_at),
            runs::current_stage.eq(None::<&str>),
            runs::current_stage_started_at.eq(None::<&str>),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update run {run_id} status to {next_status}: {e}"))?;
    } else {
        diesel::update(
            runs::table
                .filter(runs::id.eq(run_id))
                .filter(runs::status.eq_any(from_statuses)),
        )
        .set((runs::status.eq(next_status), runs::completed_at.eq(completed_at)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update run {run_id} status to {next_status}: {e}"))?;
    }
    Ok(())
}

pub fn pause_all_running(pool: &DbPool) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::update(runs::table.filter(runs::status.eq("running")))
        .set((
            runs::status.eq("paused"),
            runs::completed_at.eq(None::<&str>),
            runs::current_stage.eq(None::<&str>),
            runs::current_stage_started_at.eq(None::<&str>),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to pause running runs: {e}"))?;

    Ok(())
}

pub fn pause_run(pool: &DbPool, run_id: &str) -> Result<(), String> {
    update_run_status(
        pool,
        run_id,
        &["running", "waiting_for_input"],
        "paused",
        None,
        true,
    )
}

pub fn resume_run(pool: &DbPool, run_id: &str) -> Result<(), String> {
    update_run_status(pool, run_id, &["paused"], "running", None, false)
}

pub fn cancel_run(pool: &DbPool, run_id: &str) -> Result<(), String> {
    let now = super::now_rfc3339();
    update_run_status(
        pool,
        run_id,
        &["running", "waiting_for_input", "paused"],
        "cancelled",
        Some(&now),
        true,
    )
}
