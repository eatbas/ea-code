//! Stage execution functions: run agent stages, diff stages, and skipped stages.

use std::time::Instant;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::{dispatch_agent, emit_stage, resolve_stage_model, stage_to_str};

/// Runs an agent stage: emits Running, executes, emits Completed/Failed,
/// persists the stage result, and returns it.
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
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    let stage_str = stage_to_str(&stage);
    let model = resolve_stage_model(&stage, settings);

    match dispatch_agent(
        backend, &model, input, settings, session_id, app, run_id, stage.clone(), db,
    )
    .await
    {
        Ok(output) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage(app, run_id, &stage, &StageStatus::Completed, iteration_num);
            let _ = db::runs::insert_stage(
                db, iteration_db_id, &stage_str, "completed",
                &output.raw_text, duration_ms as i32, None,
            );
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
            emit_stage(app, run_id, &stage, &StageStatus::Failed, iteration_num);
            let _ = db::runs::insert_stage(
                db, iteration_db_id, &stage_str, "failed",
                "", duration_ms as i32, Some(&e),
            );
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

/// Runs an agent stage that is not tied to an iteration row.
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
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
    let model = resolve_stage_model(&stage, settings);

    match dispatch_agent(
        backend, &model, input, settings, session_id, app, run_id, stage.clone(), db,
    )
    .await
    {
        Ok(output) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage(app, run_id, &stage, &StageStatus::Completed, iteration_num);
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
            emit_stage(app, run_id, &stage, &StageStatus::Failed, iteration_num);
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
    emit_stage(app, run_id, &stage, &StageStatus::Skipped, iteration_num);
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
pub fn execute_diff_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    workspace_path: &str,
    db: &DbPool,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    let diff = crate::git::git_diff(workspace_path);
    let duration_ms = start.elapsed().as_millis() as u64;

    emit_stage(app, run_id, &stage, &StageStatus::Completed, iteration_num);
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
