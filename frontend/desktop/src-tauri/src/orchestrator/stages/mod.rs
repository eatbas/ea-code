//! Stage execution functions: run agent stages and skipped stages.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use tauri::AppHandle;
use tokio::time::Duration;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::{self, runs};

pub use crate::orchestrator::helpers::{
    dispatch_agent, emit_stage, emit_stage_with_duration, resolve_stage_model, wait_for_cancel,
};

mod execution;


/// Runs an agent stage with retry-on-failure support.
///
/// On failure, if `settings.agent_retry_count > 0`, the stage is re-run
/// with the prompt augmented by a `PREVIOUS ATTEMPT FAILED` hint so the
/// agent can learn from the error.  Cancellation errors are never retried.
pub async fn execute_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    session_id: Option<&str>,
    output_file: Option<&str>,
) -> StageResult {
    let max_attempts = 1 + settings.agent_retry_count;
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    let stage_str = execution::stage_to_str(&stage);
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

        let stage_for_call = stage.clone();
        let dispatch_result = if settings.agent_timeout_ms == 0 {
            tokio::select! {
                result = dispatch_agent(
                    backend,
                    &model,
                    &effective_input,
                    settings,
                    session_id,
                    app,
                    run_id,
                    stage_for_call.clone(),
                    output_file,
                ) => result,
                _ = wait_for_cancel(cancel_flag) => {
                    Err(format!("{stage_for_call:?} stage cancelled by user"))
                }
            }
        } else {
            tokio::select! {
                result = tokio::time::timeout(
                    Duration::from_millis(settings.agent_timeout_ms),
                    dispatch_agent(
                        backend,
                        &model,
                        &effective_input,
                        settings,
                        session_id,
                        app,
                        run_id,
                        stage_for_call.clone(),
                        output_file,
                    ),
                ) => {
                    match result {
                        Ok(inner) => inner,
                        Err(_) => Err(format!(
                            "{stage_for_call:?} stage timed out after {} ms",
                            settings.agent_timeout_ms
                        )),
                    }
                }
                _ = wait_for_cancel(cancel_flag) => {
                    Err(format!("{stage_for_call:?} stage cancelled by user"))
                }
            }
        };

        match dispatch_result {
            Ok(output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                emit_stage_with_duration(
                    app,
                    run_id,
                    &stage,
                    &StageStatus::Completed,
                    iteration_num,
                    Some(duration_ms),
                );
                // Append event to storage
                execution::append_stage_end_event(run_id, &stage_str, iteration_num, duration_ms, "completed");
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
    emit_stage_with_duration(
        app,
        run_id,
        &stage,
        &StageStatus::Failed,
        iteration_num,
        Some(duration_ms),
    );
    // Append event to storage
    execution::append_stage_end_event(run_id, &stage_str, iteration_num, duration_ms, "failed");
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
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
    let model = resolve_stage_model(&stage, settings);

    let dispatch_result = if settings.agent_timeout_ms == 0 {
        dispatch_agent(
            backend,
            &model,
            input,
            settings,
            session_id,
            app,
            run_id,
            stage.clone(),
            None,
        )
        .await
    } else {
        match tokio::time::timeout(
            Duration::from_millis(settings.agent_timeout_ms),
            dispatch_agent(
                backend,
                &model,
                input,
                settings,
                session_id,
                app,
                run_id,
                stage.clone(),
                None,
            ),
        )
        .await
        {
            Ok(inner) => inner,
            Err(_) => Err(format!(
                "{stage:?} stage timed out after {} ms",
                settings.agent_timeout_ms
            )),
        }
    };

    match dispatch_result {
        Ok(output) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage_with_duration(
                app,
                run_id,
                &stage,
                &StageStatus::Completed,
                iteration_num,
                Some(duration_ms),
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
            emit_stage_with_duration(
                app,
                run_id,
                &stage,
                &StageStatus::Failed,
                iteration_num,
                Some(duration_ms),
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

/// Marks a stage as skipped and persists the skip reason.
pub fn execute_skipped_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    reason: &str,
) -> StageResult {
    emit_stage(app, run_id, &stage, &StageStatus::Skipped, iteration_num);
    // Append event to storage
    let seq = match runs::next_sequence(run_id) {
        Ok(s) => s,
        Err(_) => 1,
    };
    let event = RunEvent::StageEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration: iteration_num,
        status: StageEndStatus::Skipped,
        duration_ms: 0,
        audit_verdict: None,
        verdict: None,
    };
    let _ = runs::append_event(run_id, event);

    StageResult {
        stage,
        status: StageStatus::Skipped,
        output: reason.to_string(),
        duration_ms: 0,
        error: None,
    }
}
