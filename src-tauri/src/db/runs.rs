use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::{iterations, questions, runs, stages};

use super::models::{
    IterationDetail, IterationRow, NewIteration, NewRun, NewStage, QuestionRow, RunDetail, RunRow,
    RunSummary, StageRow,
};

/// Patch object for updating typed iteration context fields.
#[derive(AsChangeset, Default)]
#[diesel(table_name = iterations)]
pub struct IterationContextPatch<'a> {
    pub enhanced_prompt: Option<&'a str>,
    pub planner_plan: Option<&'a str>,
    pub audit_verdict: Option<&'a str>,
    pub audit_reasoning: Option<&'a str>,
    pub audited_plan: Option<&'a str>,
    pub review_output: Option<&'a str>,
    pub review_user_guidance: Option<&'a str>,
    pub fix_output: Option<&'a str>,
    pub judge_output: Option<&'a str>,
    pub generate_question: Option<&'a str>,
    pub generate_answer: Option<&'a str>,
    pub fix_question: Option<&'a str>,
    pub fix_answer: Option<&'a str>,
}

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
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    diesel::insert_into(runs::table)
        .values(&NewRun {
            id,
            session_id,
            prompt,
            max_iterations,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert run: {e}"))?;

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
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
    let now = chrono::Utc::now().to_rfc3339();

    diesel::update(runs::table.find(id))
        .set((
            runs::status.eq(status),
            runs::final_verdict.eq(verdict),
            runs::error.eq(error),
            runs::completed_at.eq(&now),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to complete run: {e}"))?;

    Ok(())
}

/// Inserts a new iteration record and returns its auto-generated ID.
pub fn insert_iteration(pool: &DbPool, run_id: &str, number: i32) -> Result<i32, String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

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
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

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

/// Updates typed context fields for a specific iteration.
pub fn update_iteration_context(
    pool: &DbPool,
    run_id: &str,
    number: i32,
    patch: &IterationContextPatch<'_>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    diesel::update(
        iterations::table
            .filter(iterations::run_id.eq(run_id))
            .filter(iterations::number.eq(number)),
    )
    .set(patch)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to update iteration context: {e}"))?;

    Ok(())
}

/// Inserts a stage result record.
pub fn insert_stage(
    pool: &DbPool,
    iteration_id: i32,
    stage: &str,
    status: &str,
    output: &str,
    duration_ms: i32,
    error: Option<&str>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    diesel::insert_into(stages::table)
        .values(&NewStage {
            iteration_id,
            stage,
            status,
            output,
            duration_ms,
            error,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert stage: {e}"))?;

    Ok(())
}

/// Returns a list of run summaries for a given session.
pub fn list_for_session(pool: &DbPool, session_id: &str) -> Result<Vec<RunSummary>, String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

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

/// Returns full run detail including nested iterations, stages, and questions.
pub fn get_full(pool: &DbPool, run_id: &str) -> Result<RunDetail, String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    let run: RunRow = runs::table
        .find(run_id)
        .first(&mut conn)
        .map_err(|e| format!("Run not found: {e}"))?;

    let iter_rows: Vec<IterationRow> = iterations::table
        .filter(iterations::run_id.eq(run_id))
        .order(iterations::number.asc())
        .load(&mut conn)
        .map_err(|e| format!("Failed to load iterations: {e}"))?;

    let mut iteration_details = Vec::with_capacity(iter_rows.len());
    for iter in &iter_rows {
        let stage_rows: Vec<StageRow> = stages::table
            .filter(stages::iteration_id.eq(iter.id))
            .order(stages::created_at.asc())
            .load(&mut conn)
            .map_err(|e| format!("Failed to load stages: {e}"))?;

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
            stages: stage_rows,
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
        iterations: iteration_details,
        questions: question_rows,
    })
}

/// Stores run-level executive summary details.
pub fn update_executive_summary(
    pool: &DbPool,
    run_id: &str,
    patch: &RunExecutiveSummaryPatch<'_>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    diesel::update(runs::table.find(run_id))
        .set(patch)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update executive summary: {e}"))?;

    Ok(())
}
