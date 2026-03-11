use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::{iterations, runs, stages};

use super::models::{NewIteration, NewRun, NewStage};

/// Patch object for storing run-level executive summary information.
#[derive(AsChangeset, Default)]
#[diesel(table_name = runs)]
pub struct RunExecutiveSummaryPatch<'a> {
    pub executive_summary: Option<&'a str>,
    pub executive_summary_status: Option<&'a str>,
    pub executive_summary_error: Option<&'a str>,
    pub executive_summary_agent: Option<&'a str>,
    pub executive_summary_model: Option<&'a str>,
    pub executive_summary_generated_at: Option<&'a str>,
}

/// Inserts a new pipeline run.
pub fn insert(
    pool: &DbPool,
    id: &str,
    session_id: &str,
    prompt: &str,
    max_iterations: i32,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let now = super::now_rfc3339();

    diesel::insert_into(runs::table)
        .values(&NewRun {
            id,
            session_id,
            prompt,
            max_iterations,
            started_at: &now,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert run: {e}"))?;

    Ok(())
}

/// Updates the currently executing stage and iteration on a run row.
/// Also records when the stage started so the frontend can show an accurate timer.
pub fn update_current_stage(
    pool: &DbPool,
    id: &str,
    stage: Option<&str>,
    iteration: i32,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let started_at: Option<String> = stage.map(|_| super::now_rfc3339());

    diesel::update(runs::table.find(id))
        .set((
            runs::current_stage.eq(stage),
            runs::current_iteration.eq(iteration),
            runs::current_stage_started_at.eq(&started_at),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update current stage: {e}"))?;

    Ok(())
}

/// Updates a run's final status on completion, failure, or cancellation.
pub fn complete(
    pool: &DbPool,
    id: &str,
    status: &str,
    verdict: Option<&str>,
    error: Option<&str>,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let now = super::now_rfc3339();

    diesel::update(runs::table.find(id))
        .set((
            runs::status.eq(status),
            runs::final_verdict.eq(verdict),
            runs::error.eq(error),
            runs::completed_at.eq(&now),
            runs::current_stage.eq(None::<&str>),
            runs::current_stage_started_at.eq(None::<&str>),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to complete run: {e}"))?;

    Ok(())
}

/// Inserts a new iteration record and returns its auto-generated ID.
pub fn insert_iteration(pool: &DbPool, run_id: &str, number: i32) -> Result<i32, String> {
    let mut conn = super::get_conn(pool)?;

    diesel::insert_into(iterations::table)
        .values(&NewIteration { run_id, number })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert iteration: {e}"))?;

    // Retrieve the generated ID
    iterations::table
        .filter(iterations::run_id.eq(run_id))
        .filter(iterations::number.eq(number))
        .select(iterations::id)
        .first::<i32>(&mut conn)
        .map_err(|e| format!("Failed to get iteration id: {e}"))
}

/// Updates an iteration's verdict and judge reasoning.
pub fn update_iteration_verdict(
    pool: &DbPool,
    run_id: &str,
    number: i32,
    verdict: Option<&str>,
    judge_reasoning: Option<&str>,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::update(
        iterations::table
            .filter(iterations::run_id.eq(run_id))
            .filter(iterations::number.eq(number)),
    )
    .set((
        iterations::verdict.eq(verdict),
        iterations::judge_reasoning.eq(judge_reasoning),
    ))
    .execute(&mut conn)
    .map_err(|e| format!("Failed to update iteration verdict: {e}"))?;

    Ok(())
}

/// Inserts a stage result record, truncating oversized output.
pub fn insert_stage(
    pool: &DbPool,
    iteration_id: i32,
    stage: &str,
    status: &str,
    output: &str,
    duration_ms: i32,
    error: Option<&str>,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let capped = super::truncate_for_storage(output);

    diesel::insert_into(stages::table)
        .values(&NewStage {
            iteration_id,
            stage,
            status,
            output: &capped,
            duration_ms,
            error,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert stage: {e}"))?;

    Ok(())
}

/// Updates plan approval status for an iteration (used by the plan gate).
pub fn update_iteration_plan_approval(
    pool: &DbPool,
    run_id: &str,
    number: i32,
    approval: &str,
    revision_count: i32,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::update(
        iterations::table
            .filter(iterations::run_id.eq(run_id))
            .filter(iterations::number.eq(number)),
    )
    .set((
        iterations::plan_approval.eq(Some(approval)),
        iterations::plan_revision_count.eq(revision_count),
    ))
    .execute(&mut conn)
    .map_err(|e| format!("Failed to update plan approval: {e}"))?;

    Ok(())
}

/// Stores run-level executive summary details.
pub fn update_executive_summary(
    pool: &DbPool,
    run_id: &str,
    patch: &RunExecutiveSummaryPatch<'_>,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::update(runs::table.find(run_id))
        .set(patch)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update executive summary: {e}"))?;

    Ok(())
}
