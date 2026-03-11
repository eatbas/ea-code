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
            enhanced_prompt: iter.enhanced_prompt.clone(),
            planner_plan: iter.planner_plan.clone(),
            audit_verdict: iter.audit_verdict.clone(),
            audit_reasoning: iter.audit_reasoning.clone(),
            audited_plan: iter.audited_plan.clone(),
            review_output: iter.review_output.clone(),
            review_user_guidance: iter.review_user_guidance.clone(),
            fix_output: iter.fix_output.clone(),
            judge_output: iter.judge_output.clone(),
            generate_question: iter.generate_question.clone(),
            generate_answer: iter.generate_answer.clone(),
            fix_question: iter.fix_question.clone(),
            fix_answer: iter.fix_answer.clone(),
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
