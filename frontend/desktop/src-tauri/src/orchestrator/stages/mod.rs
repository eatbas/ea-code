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
    dispatch_agent, emit_stage, emit_stage_with_duration, resolve_stage_model, wait_for_interrupt,
    wait_if_paused, RunInterrupt,
};

fn validate_text_stage_output(stage: &PipelineStage, output: &str) -> Result<(), String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("agent returned empty output".to_string());
    }

    if looks_like_cli_result_envelope(trimmed) {
        return Err(
            "agent returned a CLI result envelope instead of the final artefact".to_string(),
        );
    }

    if looks_like_process_preamble(trimmed) {
        return Err("agent returned a process preamble instead of the final artefact".to_string());
    }

    match stage {
        PipelineStage::CodeReviewer
        | PipelineStage::ExtraReviewer(_)
        | PipelineStage::ReviewMerge => {
            if !trimmed.contains("## BLOCKERS") || !trimmed.contains("Verdict:") {
                return Err("review output did not match the required review schema".to_string());
            }
        }
        PipelineStage::Judge => {
            let first_non_empty = trimmed
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("");
            let first_trimmed = first_non_empty.trim();
            let is_complete = first_trimmed.eq_ignore_ascii_case("COMPLETE");
            let is_not_complete = first_trimmed.eq_ignore_ascii_case("NOT COMPLETE");
            let has_verdict_line = trimmed.lines().any(|line| {
                let t = line.trim();
                t.len() >= 8 && t[..8].eq_ignore_ascii_case("VERDICT:")
            });

            if !is_complete && !is_not_complete && !has_verdict_line {
                return Err("judge output did not include a parseable verdict line".to_string());
            }
        }
        _ => {}
    }

    Ok(())
}

fn looks_like_cli_result_envelope(text: &str) -> bool {
    let compact = text.trim_start();
    compact.starts_with("{\"type\":\"result\"")
        || compact.starts_with("{\"subtype\":")
        || (compact.starts_with('{')
            && compact.contains("\"type\":\"result\"")
            && compact.contains("\"stop_reason\""))
}

/// Phrases that indicate the agent is narrating its process rather than
/// returning a final artefact. Stored as a static to avoid re-allocation.
static PREAMBLE_PREFIXES: &[&str] = &[
    "i'll start by",
    "i will start by",
    "i’ll start by",
    "let me start by",
    "first, i'll",
    "first, i’ll",
    "first i'll",
    "first i’ll",
    "i'm going to start by",
    "i’m going to start by",
    "i am going to start by",
];

fn looks_like_process_preamble(text: &str) -> bool {
    let first_non_empty = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    let lower = first_non_empty.to_lowercase();
    PREAMBLE_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

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
            Ok(output) => {
                if matches!(stage.execution_intent(), StageExecutionIntent::Text) {
                    if let Err(validation_error) =
                        validate_text_stage_output(&stage, &output.raw_text)
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
    output_file: Option<&str>,
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
            output_file,
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
                output_file,
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
            if matches!(stage.execution_intent(), StageExecutionIntent::Text) {
                if let Err(validation_error) = validate_text_stage_output(&stage, &output.raw_text)
                {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    emit_stage_with_duration(
                        app,
                        run_id,
                        &stage,
                        &StageStatus::Failed,
                        iteration_num,
                        Some(duration_ms),
                    );
                    return StageResult {
                        stage,
                        status: StageStatus::Failed,
                        output: String::new(),
                        duration_ms,
                        error: Some(validation_error),
                    };
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

#[cfg(test)]
mod tests {
    use super::{
        looks_like_cli_result_envelope, looks_like_process_preamble, validate_text_stage_output,
    };
    use crate::models::PipelineStage;

    #[test]
    fn rejects_cli_result_envelope() {
        let raw =
            r#"{"type":"result","subtype":"success","result":"hello","stop_reason":"end_turn"}"#;
        assert!(looks_like_cli_result_envelope(raw));
        assert!(validate_text_stage_output(&PipelineStage::ExtraPlan(1), raw).is_err());
    }

    #[test]
    fn rejects_process_preamble() {
        let raw = "I'll start by exploring the codebase to understand its structure.";
        assert!(looks_like_process_preamble(raw));
        assert!(validate_text_stage_output(&PipelineStage::Plan, raw).is_err());
    }

    #[test]
    fn accepts_structured_review_output() {
        let raw = "## BLOCKERS\n- None.\n\n## WARNINGS\n- None.\n\n## NITS\n- None.\n\n## TESTS\n- Status: not run\n- Commands: None.\n\n## TEST RESULTS\n- None.\n\n## TEST GAPS\n- Add coverage.\n\n## ACTION ITEMS\n- None.\n\n## SUMMARY\nVerdict: PASS";
        assert!(validate_text_stage_output(&PipelineStage::CodeReviewer, raw).is_ok());
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
