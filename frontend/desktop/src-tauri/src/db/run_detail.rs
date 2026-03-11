//! Run detail and listing queries for the history views.

use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::{iterations, questions, runs, stages};

use super::models::{
    IterationDetail, IterationRow, QuestionRow, RunDetail, RunRow, RunSummary, StageRow,
};

/// Returns a list of run summaries for a given session.
pub fn list_for_session(pool: &DbPool, session_id: &str) -> Result<Vec<RunSummary>, String> {
    let mut conn = super::get_conn(pool)?;

    let rows: Vec<RunRow> = runs::table
        .filter(runs::session_id.eq(session_id))
        .order(runs::started_at.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to list runs: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| RunSummary {
            id: r.id,
            prompt: r.prompt,
            status: r.status,
            final_verdict: r.final_verdict,
            executive_summary: r.executive_summary,
            started_at: r.started_at,
            completed_at: r.completed_at,
        })
        .collect())
}

/// Returns recent run summaries for a session, newest first then re-ordered
/// to oldest-first for chronological prompt context.
pub fn list_recent_for_session(
    pool: &DbPool,
    session_id: &str,
    limit: i64,
    exclude_run_id: Option<&str>,
) -> Result<Vec<RunSummary>, String> {
    let mut conn = super::get_conn(pool)?;

    let mut query = runs::table
        .filter(runs::session_id.eq(session_id))
        .into_boxed();
    if let Some(run_id) = exclude_run_id {
        query = query.filter(runs::id.ne(run_id));
    }

    let rows: Vec<RunRow> = query
        .order(runs::started_at.desc())
        .limit(limit)
        .load(&mut conn)
        .map_err(|e| format!("Failed to list recent runs: {e}"))?;

    let mut summaries = rows
        .into_iter()
        .map(|r| RunSummary {
            id: r.id,
            prompt: r.prompt,
            status: r.status,
            final_verdict: r.final_verdict,
            executive_summary: r.executive_summary,
            started_at: r.started_at,
            completed_at: r.completed_at,
        })
        .collect::<Vec<_>>();

    summaries.reverse();
    Ok(summaries)
}

/// Returns the total number of runs in a session.
pub fn count_for_session(pool: &DbPool, session_id: &str) -> Result<i64, String> {
    let mut conn = super::get_conn(pool)?;

    runs::table
        .filter(runs::session_id.eq(session_id))
        .count()
        .get_result(&mut conn)
        .map_err(|e| format!("Failed to count runs: {e}"))
}

/// Returns paginated, fully-hydrated run details for a session.
///
/// Loads the most recent `limit` runs (offset by `offset` from the newest),
/// then returns them in chronological order (oldest first). Uses batch queries
/// to avoid N+1: 4 queries total regardless of run count.
pub fn list_full_for_session(
    pool: &DbPool,
    session_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<RunDetail>, String> {
    let mut conn = super::get_conn(pool)?;

    // Load the page of runs (newest first so offset skips the most recent)
    let mut run_rows: Vec<RunRow> = runs::table
        .filter(runs::session_id.eq(session_id))
        .order(runs::started_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| format!("Failed to list runs: {e}"))?;

    if run_rows.is_empty() {
        return Ok(Vec::new());
    }

    // Reverse to chronological order (oldest first) for display
    run_rows.reverse();

    let run_ids: Vec<&str> = run_rows.iter().map(|r| r.id.as_str()).collect();

    // Batch-load iterations for all runs
    let iter_rows: Vec<IterationRow> = iterations::table
        .filter(iterations::run_id.eq_any(&run_ids))
        .order(iterations::number.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to load iterations: {e}"))?;

    // Batch-load stages for all iterations
    let iter_db_ids: Vec<i32> = iter_rows.iter().map(|i| i.id).collect();
    let all_stages: Vec<StageRow> = if iter_db_ids.is_empty() {
        Vec::new()
    } else {
        stages::table
            .filter(stages::iteration_id.eq_any(&iter_db_ids))
            .order(stages::created_at.asc())
            .load(&mut conn)
            .map_err(|e| format!("Failed to load stages: {e}"))?
    };

    // Batch-load questions for all runs
    let question_rows: Vec<QuestionRow> = questions::table
        .filter(questions::run_id.eq_any(&run_ids))
        .order(questions::asked_at.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to load questions: {e}"))?;

    // Group stages by iteration_id
    let mut stages_by_iter: std::collections::HashMap<i32, Vec<StageRow>> =
        std::collections::HashMap::new();
    for stage in all_stages {
        stages_by_iter
            .entry(stage.iteration_id)
            .or_default()
            .push(stage);
    }

    // Group iterations by run_id
    let mut iters_by_run: std::collections::HashMap<&str, Vec<&IterationRow>> =
        std::collections::HashMap::new();
    for iter in &iter_rows {
        iters_by_run
            .entry(iter.run_id.as_str())
            .or_default()
            .push(iter);
    }

    // Group questions by run_id (owned key to avoid lifetime issues)
    let mut questions_by_run: std::collections::HashMap<String, Vec<QuestionRow>> =
        std::collections::HashMap::new();
    for q in question_rows {
        questions_by_run
            .entry(q.run_id.clone())
            .or_default()
            .push(q);
    }

    // Assemble RunDetail structs
    let details = run_rows
        .into_iter()
        .map(|run| {
            let iteration_details = iters_by_run
                .get(run.id.as_str())
                .map(|iters| {
                    iters
                        .iter()
                        .map(|iter| IterationDetail {
                            number: iter.number,
                            verdict: iter.verdict.clone(),
                            judge_reasoning: iter.judge_reasoning.clone(),
                            stages: stages_by_iter.remove(&iter.id).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let run_questions = questions_by_run
                .remove(run.id.as_str())
                .unwrap_or_default();

            RunDetail {
                id: run.id,
                prompt: run.prompt,
                status: run.status,
                final_verdict: run.final_verdict,
                error: run.error,
                executive_summary: run.executive_summary,
                executive_summary_status: run.executive_summary_status,
                executive_summary_error: run.executive_summary_error,
                executive_summary_agent: run.executive_summary_agent,
                executive_summary_model: run.executive_summary_model,
                executive_summary_generated_at: run.executive_summary_generated_at,
                max_iterations: run.max_iterations,
                started_at: run.started_at,
                completed_at: run.completed_at,
                current_stage: run.current_stage,
                current_iteration: run.current_iteration,
                current_stage_started_at: run.current_stage_started_at,
                iterations: iteration_details,
                questions: run_questions,
            }
        })
        .collect();

    Ok(details)
}

/// Returns full run detail including nested iterations, stages, and questions.
pub fn get_full(pool: &DbPool, run_id: &str) -> Result<RunDetail, String> {
    let mut conn = super::get_conn(pool)?;

    let run: RunRow = runs::table
        .find(run_id)
        .first(&mut conn)
        .map_err(|e| format!("Run not found: {e}"))?;

    let iter_rows: Vec<IterationRow> = iterations::table
        .filter(iterations::run_id.eq(run_id))
        .order(iterations::number.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to load iterations: {e}"))?;

    // Batch-load all stages for every iteration in one query (avoids N+1)
    let all_stages: Vec<StageRow> = stages::table
        .filter(
            stages::iteration_id
                .eq_any(iter_rows.iter().map(|i| i.id).collect::<Vec<_>>()),
        )
        .order(stages::created_at.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to load stages: {e}"))?;

    // Group stages by iteration_id for lookup
    let mut stages_by_iter: std::collections::HashMap<i32, Vec<StageRow>> =
        std::collections::HashMap::new();
    for stage in all_stages {
        stages_by_iter
            .entry(stage.iteration_id)
            .or_default()
            .push(stage);
    }

    let mut iteration_details = Vec::with_capacity(iter_rows.len());
    for iter in &iter_rows {
        iteration_details.push(IterationDetail {
            number: iter.number,
            verdict: iter.verdict.clone(),
            judge_reasoning: iter.judge_reasoning.clone(),
            stages: stages_by_iter.remove(&iter.id).unwrap_or_default(),
        });
    }

    let question_rows: Vec<QuestionRow> = questions::table
        .filter(questions::run_id.eq(run_id))
        .order(questions::asked_at.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to load questions: {e}"))?;

    Ok(RunDetail {
        id: run.id,
        prompt: run.prompt,
        status: run.status,
        final_verdict: run.final_verdict,
        error: run.error,
        executive_summary: run.executive_summary,
        executive_summary_status: run.executive_summary_status,
        executive_summary_error: run.executive_summary_error,
        executive_summary_agent: run.executive_summary_agent,
        executive_summary_model: run.executive_summary_model,
        executive_summary_generated_at: run.executive_summary_generated_at,
        max_iterations: run.max_iterations,
        started_at: run.started_at,
        completed_at: run.completed_at,
        current_stage: run.current_stage,
        current_iteration: run.current_iteration,
        current_stage_started_at: run.current_stage_started_at,
        iterations: iteration_details,
        questions: question_rows,
    })
}
