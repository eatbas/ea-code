use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::agents::{run_claude, run_codex, run_gemini, AgentInput, AgentOutput};
use crate::events::*;
use crate::git;
use crate::models::*;

/// Returns the current time as epoch milliseconds (string).
fn epoch_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

/// Dispatches to the appropriate agent runner based on the backend setting.
async fn dispatch_agent(
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    match backend {
        AgentBackend::Claude => {
            run_claude(input, &settings.claude_path, app, run_id, stage).await
        }
        AgentBackend::Codex => {
            run_codex(input, &settings.codex_path, app, run_id, stage).await
        }
        AgentBackend::Gemini => {
            run_gemini(input, &settings.gemini_path, app, run_id, stage).await
        }
    }
}

/// Emits a stage status transition event.
fn emit_stage(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
) {
    let _ = app.emit(
        "pipeline:stage",
        PipelineStagePayload {
            run_id: run_id.to_string(),
            stage: stage.clone(),
            status: status.clone(),
            iteration,
        },
    );
}

/// Emits an artefact event (diff, review, validation, or judge output).
fn emit_artifact(app: &AppHandle, run_id: &str, kind: &str, content: &str, iteration: u32) {
    let _ = app.emit(
        "pipeline:artifact",
        PipelineArtifactPayload {
            run_id: run_id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            iteration,
        },
    );
}

/// Runs an agent stage: emits Running, executes, emits Completed/Failed,
/// and returns the `StageResult`.
async fn execute_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    match dispatch_agent(backend, input, settings, app, run_id, stage.clone()).await {
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

/// Captures a git diff and wraps it in a `StageResult`.
fn execute_diff_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    workspace_path: &str,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    let diff = git::git_diff(workspace_path);
    let duration_ms = start.elapsed().as_millis() as u64;

    emit_stage(app, run_id, &stage, &StageStatus::Completed, iteration_num);
    emit_artifact(app, run_id, "diff", &diff, iteration_num);

    StageResult {
        stage,
        status: StageStatus::Completed,
        output: diff,
        duration_ms,
        error: None,
    }
}

/// Parses the judge verdict from raw output text.
/// The first line must be exactly "COMPLETE" or "NOT COMPLETE"; anything
/// else is treated as NOT COMPLETE.
fn parse_judge_verdict(output: &str) -> (JudgeVerdict, String) {
    let first_line = output.lines().next().unwrap_or("").trim();
    let reasoning = output
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");

    let verdict = if first_line == "COMPLETE" {
        JudgeVerdict::Complete
    } else {
        JudgeVerdict::NotComplete
    };

    (verdict, reasoning)
}

/// Returns `true` if the cancel flag has been set.
fn is_cancelled(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

/// Runs the full orchestration pipeline:
///   generate → diff → review → fix → diff → validate → judge → loop
pub async fn run_pipeline(
    app: AppHandle,
    request: PipelineRequest,
    settings: AppSettings,
    cancel_flag: Arc<AtomicBool>,
) -> Result<PipelineRun, String> {
    let run_id = Uuid::new_v4().to_string();
    let pipeline_start = Instant::now();

    let mut run = PipelineRun {
        id: run_id.clone(),
        status: PipelineStatus::Running,
        prompt: request.prompt.clone(),
        workspace_path: request.workspace_path.clone(),
        iterations: Vec::new(),
        current_iteration: 0,
        current_stage: None,
        max_iterations: settings.max_iterations,
        started_at: Some(epoch_millis()),
        completed_at: None,
        final_verdict: None,
        error: None,
    };

    // Emit pipeline:started
    let _ = app.emit(
        "pipeline:started",
        PipelineStartedPayload {
            run_id: run_id.clone(),
            prompt: request.prompt.clone(),
            workspace_path: request.workspace_path.clone(),
        },
    );

    for iter_num in 1..=settings.max_iterations {
        if is_cancelled(&cancel_flag) {
            run.status = PipelineStatus::Cancelled;
            break;
        }

        run.current_iteration = iter_num;
        let mut stages: Vec<StageResult> = Vec::new();

        // --- 1. Generate ---
        let gen_input = AgentInput {
            prompt: request.prompt.clone(),
            context: None,
            diff: None,
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Generate);
        let gen_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::Generate,
            &settings.generator_agent,
            &gen_input,
            &settings,
        )
        .await;
        let gen_failed = gen_result.status == StageStatus::Failed;
        stages.push(gen_result);
        if gen_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if gen_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Generator stage failed".to_string());
            }
            break;
        }

        // --- 2. Diff after generate ---
        run.current_stage = Some(PipelineStage::DiffAfterGenerate);
        let diff1 = execute_diff_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::DiffAfterGenerate,
            &request.workspace_path,
        );
        let diff1_output = diff1.output.clone();
        stages.push(diff1);

        if is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            run.status = PipelineStatus::Cancelled;
            break;
        }

        // --- 3. Review ---
        let review_input = AgentInput {
            prompt: request.prompt.clone(),
            context: None,
            diff: Some(diff1_output.clone()),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Review);
        let review_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::Review,
            &settings.reviewer_agent,
            &review_input,
            &settings,
        )
        .await;
        let review_output = review_result.output.clone();
        let review_failed = review_result.status == StageStatus::Failed;
        emit_artifact(&app, &run_id, "review", &review_output, iter_num);
        stages.push(review_result);
        if review_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if review_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Reviewer stage failed".to_string());
            }
            break;
        }

        // --- 4. Fix ---
        let fix_input = AgentInput {
            prompt: request.prompt.clone(),
            context: Some(review_output),
            diff: Some(diff1_output),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Fix);
        let fix_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::Fix,
            &settings.fixer_agent,
            &fix_input,
            &settings,
        )
        .await;
        let fix_failed = fix_result.status == StageStatus::Failed;
        stages.push(fix_result);
        if fix_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if fix_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Fixer stage failed".to_string());
            }
            break;
        }

        // --- 5. Diff after fix ---
        run.current_stage = Some(PipelineStage::DiffAfterFix);
        let diff2 = execute_diff_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::DiffAfterFix,
            &request.workspace_path,
        );
        let diff2_output = diff2.output.clone();
        stages.push(diff2);

        if is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            run.status = PipelineStatus::Cancelled;
            break;
        }

        // --- 6. Validate ---
        let validate_input = AgentInput {
            prompt: request.prompt.clone(),
            context: None,
            diff: Some(diff2_output.clone()),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Validate);
        let validate_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::Validate,
            &settings.validator_agent,
            &validate_input,
            &settings,
        )
        .await;
        let validation_output = validate_result.output.clone();
        let validate_failed = validate_result.status == StageStatus::Failed;
        emit_artifact(&app, &run_id, "validation", &validation_output, iter_num);
        stages.push(validate_result);
        if validate_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if validate_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Validator stage failed".to_string());
            }
            break;
        }

        // --- 7. Judge ---
        let judge_input = AgentInput {
            prompt: request.prompt.clone(),
            context: Some(validation_output),
            diff: Some(diff2_output),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Judge);
        let judge_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            PipelineStage::Judge,
            &settings.final_judge_agent,
            &judge_input,
            &settings,
        )
        .await;
        let judge_output = judge_result.output.clone();
        let judge_failed = judge_result.status == StageStatus::Failed;
        emit_artifact(&app, &run_id, "judge", &judge_output, iter_num);
        stages.push(judge_result);

        if judge_failed {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            run.status = PipelineStatus::Failed;
            run.error = Some("Judge stage failed".to_string());
            break;
        }

        // --- Parse verdict ---
        let (verdict, reasoning) = parse_judge_verdict(&judge_output);

        run.iterations.push(Iteration {
            number: iter_num,
            stages,
            verdict: Some(verdict.clone()),
            judge_reasoning: Some(reasoning),
        });

        if verdict == JudgeVerdict::Complete {
            run.final_verdict = Some(JudgeVerdict::Complete);
            run.status = PipelineStatus::Completed;
            break;
        }

        // If this was the last iteration and still NOT COMPLETE, mark as completed
        // with the NOT COMPLETE verdict.
        if iter_num == settings.max_iterations {
            run.final_verdict = Some(JudgeVerdict::NotComplete);
            run.status = PipelineStatus::Completed;
        }
    }

    // Handle cancellation status
    if is_cancelled(&cancel_flag) {
        run.status = PipelineStatus::Cancelled;
    }

    let total_duration_ms = pipeline_start.elapsed().as_millis() as u64;
    run.completed_at = Some(epoch_millis());
    run.current_stage = None;

    // Emit completion or error event
    match &run.status {
        PipelineStatus::Completed => {
            let _ = app.emit(
                "pipeline:completed",
                PipelineCompletedPayload {
                    run_id: run_id.clone(),
                    verdict: run
                        .final_verdict
                        .clone()
                        .unwrap_or(JudgeVerdict::NotComplete),
                    total_iterations: run.current_iteration,
                    duration_ms: total_duration_ms,
                },
            );
        }
        PipelineStatus::Failed => {
            let _ = app.emit(
                "pipeline:error",
                PipelineErrorPayload {
                    run_id: run_id.clone(),
                    stage: None,
                    message: run
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unknown error".to_string()),
                },
            );
        }
        PipelineStatus::Cancelled => {
            let _ = app.emit(
                "pipeline:error",
                PipelineErrorPayload {
                    run_id: run_id.clone(),
                    stage: None,
                    message: "Pipeline cancelled by user".to_string(),
                },
            );
        }
        _ => {}
    }

    Ok(run)
}
