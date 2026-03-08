use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::questions;

use super::models::NewQuestion;

/// Inserts a question record when the pipeline pauses for user input.
pub fn insert(
    pool: &DbPool,
    id: &str,
    run_id: &str,
    stage: &str,
    iteration: i32,
    question_text: &str,
    agent_output: &str,
    optional: bool,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::insert_into(questions::table)
        .values(&NewQuestion {
            id,
            run_id,
            stage,
            iteration,
            question_text,
            agent_output,
            optional,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert question: {e}"))?;

    Ok(())
}

/// Records the user's answer (or skip) for a previously asked question.
pub fn record_answer(
    pool: &DbPool,
    question_id: &str,
    answer: Option<&str>,
    skipped: bool,
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let now = super::now_rfc3339();

    diesel::update(questions::table.find(question_id))
        .set((
            questions::answer.eq(answer),
            questions::skipped.eq(skipped),
            questions::answered_at.eq(&now),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to record answer: {e}"))?;

    Ok(())
}
