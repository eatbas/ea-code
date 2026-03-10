//! Stage execution functions: run agent stages, diff stages, and skipped stages.

use std::time::Instant;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::{dispatch_agent, emit_stage, emit_stage_with_duration, resolve_stage_model, stage_to_str};

/// Runs an agent stage with retry-on-failure support.
///
/// On failure, if `settings.agent_retry_count > 0`, the stage is re-run
/// with the prompt augmented by a `PREVIOUS ATTEMPT FAILED` hint so the
/// agent can learn from the error.  Cancellation errors are never retried.
pub async fn execute_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    db: &DbPool,
) -> StageResult {
    let max_attempts = 1 + settings.agent_retry_count;
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num, db);

    let stage_str = stage_to_str(&stage);
    let model = resolve_stage_model(&stage, settings);

    let mut last_error = String::new();

    for attempt in 0..max_attempts {
        // On retry, augment the prompt with the failure hint.
        let effective_input = if attempt == 0 {
            input.clone()
        } else {
            let augmented_prompt = format!(
                "{}\n\n\
                 PREVIOUS ATTEMPT FAILED (attempt {prev} of {max}):\n\
                 {err}\n\n\
                 Please try a different approach to resolve the issue.",
                input.prompt,
                prev = attempt,
                max = max_attempts,
                err = last_error,
            );
            AgentInput {
                prompt: augmented_prompt,
                context: input.context.clone(),
                workspace_path: input.workspace_path.clone(),
            }
        };

        match dispatch_agent(
            backend, &model, &effective_input, settings, session_id, app,
            run_id, stage.clone(), db,
        )
        .await
        {
            Ok(output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                emit_stage_with_duration(app, run_id, &stage, &StageStatus::Completed, iteration_num, Some(duration_ms), db);
                let _ = db::runs::insert_stage(
                    db, iteration_db_id, &stage_str, "completed",
                    &output.raw_text, duration_ms as i32, None,
                );
                return StageResult {
                    stage,
                    status: StageStatus::Completed,
                    output: output.raw_text,
                    duration_ms,
                    error: None,
                };
            }
            Err(e) => {
                last_error = e;
                // Don't retry cancellation errors.
                if last_error.to_lowercase().contains("cancel")
                    || last_error.to_lowercase().contains("abort")
                {
                    break;
                }
            }
        }
    }

    // All attempts exhausted — return the last error.
    let duration_ms = start.elapsed().as_millis() as u64;
    emit_stage_with_duration(app, run_id, &stage, &StageStatus::Failed, iteration_num, Some(duration_ms), db);
    let _ = db::runs::insert_stage(
        db, iteration_db_id, &stage_str, "failed",
        "", duration_ms as i32, Some(&last_error),
    );
    StageResult {
        stage,
        status: StageStatus::Failed,
        output: String::new(),
        duration_ms,
        error: Some(last_error),
    }
}

/// Runs an agent stage that is not tied to an iteration row.
#[allow(dead_code)]
pub async fn execute_run_level_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    db: &DbPool,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num, db);
    let model = resolve_stage_model(&stage, settings);

    match dispatch_agent(
        backend, &model, input, settings, session_id, app, run_id, stage.clone(), db,
    )
    .await
    {
        Ok(output) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage_with_duration(app, run_id, &stage, &StageStatus::Completed, iteration_num, Some(duration_ms), db);
            StageResult {
                stage,
                status: StageStatus::Completed,
                output: output.raw_text,
                duration_ms,
                error: None,
            }
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage_with_duration(app, run_id, &stage, &StageStatus::Failed, iteration_num, Some(duration_ms), db);
            StageResult {
                stage,
                status: StageStatus::Failed,
                output: String::new(),
                duration_ms,
                error: Some(e),
            }
        }
    }
}

/// Marks a stage as skipped and persists the skip reason.
pub fn execute_skipped_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    reason: &str,
    db: &DbPool,
) -> StageResult {
    emit_stage(app, run_id, &stage, &StageStatus::Skipped, iteration_num, db);
    let stage_str = stage_to_str(&stage);
    let _ = db::runs::insert_stage(db, iteration_db_id, &stage_str, "skipped", reason, 0, None);
    StageResult {
        stage,
        status: StageStatus::Skipped,
        output: reason.to_string(),
        duration_ms: 0,
        error: None,
    }
}

/// Captures a git diff and wraps it in a `StageResult`, persisting to DB.
pub async fn execute_diff_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    workspace_path: &str,
    db: &DbPool,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num, db);

    let diff = crate::git::git_diff(workspace_path).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    emit_stage_with_duration(app, run_id, &stage, &StageStatus::Completed, iteration_num, Some(duration_ms), db);
    super::helpers::emit_artifact(app, run_id, "diff", &diff, iteration_num, db);

    let stage_str = stage_to_str(&stage);
    let _ = db::runs::insert_stage(
        db, iteration_db_id, &stage_str, "completed",
        &diff, duration_ms as i32, None,
    );

    StageResult {
        stage,
        status: StageStatus::Completed,
        output: diff,
        duration_ms,
        error: None,
    }
}
