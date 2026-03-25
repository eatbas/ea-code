//! Stage execution functions: run agent stages and skipped stages.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use tauri::AppHandle;
use tokio::time::Duration;

use crate::agents::AgentInput;
use crate::models::*;

pub use crate::orchestrator::helpers::{
    dispatch_agent, emit_stage, emit_stage_with_duration, resolve_stage_model, wait_for_interrupt,
    wait_if_paused, RunInterrupt,
};

mod run_level;
mod validation;

pub use run_level::execute_run_level_agent_stage;

use validation::validate_text_stage_output;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseHandling {
    ResumeWithinStage,
    ReturnPausedError,
}

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
    pause_flag: &Arc<AtomicBool>,
    pause_handling: PauseHandling,
    session_id: Option<&str>,
    output_file: Option<&str>,
    cli_session_ref: Option<&str>,
    abort_flag: Option<Arc<AtomicBool>>,
) -> StageResult {
    let max_attempts = 1 + settings.agent_retry_count;
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
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

        let dispatch_result = loop {
            let stage_for_call = stage.clone();
            let abort_ref = abort_flag.clone();
            let dispatch_future = async {
                if settings.agent_timeout_ms == 0 {
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
                        cli_session_ref,
                        abort_ref,
                    )
                    .await
                } else {
                    match tokio::time::timeout(
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
                            cli_session_ref,
                            abort_ref,
                        ),
                    )
                    .await
                    {
                        Ok(inner) => inner,
                        Err(_) => Err(format!(
                            "{stage_for_call:?} stage timed out after {} ms",
                            settings.agent_timeout_ms
                        )),
                    }
                }
            };

            let outcome = tokio::select! {
                result = dispatch_future => Ok(result),
                interrupt = wait_for_interrupt(pause_flag, cancel_flag) => Err(interrupt),
            };

            match outcome {
                Ok(result) => break result,
                Err(RunInterrupt::Cancel) => {
                    break Err(format!("{stage:?} stage cancelled by user"));
                }
                Err(RunInterrupt::Pause) => {
                    if wait_if_paused(pause_flag, cancel_flag).await {
                        break Err(format!("{stage:?} stage cancelled by user"));
                    }
                    match pause_handling {
                        PauseHandling::ResumeWithinStage => {
                            emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
                        }
                        PauseHandling::ReturnPausedError => {
                            break Err(format!("{stage:?} stage paused by user"));
                        }
                    }
                }
            }
        };

        match dispatch_result {
            Ok(dr) => {
                if matches!(stage.execution_intent(), StageExecutionIntent::Text) {
                    if let Err(validation_error) =
                        validate_text_stage_output(&stage, &dr.output.raw_text)
                    {
                        last_error = validation_error;
                        continue;
                    }
                }
                let duration_ms = start.elapsed().as_millis() as u64;
                emit_stage_with_duration(
                    app,
                    run_id,
                    &stage,
                    &StageStatus::Completed,
                    iteration_num,
                    Some(duration_ms),
                );
                return StageResult {
                    stage,
                    status: StageStatus::Completed,
                    output: dr.output.raw_text,
                    duration_ms,
                    error: None,
                    backend: Some(backend.clone()),
                    provider_session_ref: dr.provider_session_ref,
                    session_pair: None,
                    resumed: Some(cli_session_ref.is_some()),
                };
            }
            Err(e) => {
                last_error = e;
                // Don't retry cancellation errors.
                if last_error.to_lowercase().contains("cancel")
                    || last_error.to_lowercase().contains("abort")
                    || last_error.to_lowercase().contains("paused by user")
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
    StageResult {
        stage,
        status: StageStatus::Failed,
        output: String::new(),
        duration_ms,
        error: Some(last_error),
        backend: Some(backend.clone()),
        provider_session_ref: None,
        session_pair: None,
        resumed: None,
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
    // Skipped-stage events are not persisted to the event log because
    // workspace_path and session_id are not available at this level.
    // The skip is visible via the frontend event emitted above.

    StageResult {
        stage,
        status: StageStatus::Skipped,
        output: reason.to_string(),
        duration_ms: 0,
        error: None,
        backend: None,
        provider_session_ref: None,
        session_pair: None,
        resumed: None,
    }
}
