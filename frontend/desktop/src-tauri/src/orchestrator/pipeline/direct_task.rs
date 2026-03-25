//! Direct task execution logic.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use tauri::AppHandle;
use tokio::time::Duration;

use crate::agents::AgentInput;
use crate::models::{RunEvent, StageEndStatus, *};
use crate::storage;
use crate::storage::runs;
use crate::storage::sessions;

use crate::orchestrator::helpers::{
    dispatch_agent, emit_stage, emit_stage_with_duration, is_cancelled, push_cancel_iteration,
    wait_for_interrupt, wait_if_paused, RunInterrupt,
};

/// Executes a single agent call directly, bypassing the full pipeline.
pub async fn run_direct_task(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    run_id: &str,
    session_id: &str,
    shared_context: &str,
    run: &mut PipelineRun,
) -> Result<(), String> {
    let backend = request
        .direct_task_agent
        .as_ref()
        .ok_or_else(|| "Direct task mode requires an agent backend".to_string())?;
    let model = request.direct_task_model.as_deref().unwrap_or("");

    run.current_iteration = 1;
    run.current_stage = Some(PipelineStage::DirectTask);

    let input = AgentInput {
        prompt: request.prompt.clone(),
        context: if shared_context.trim().is_empty() {
            None
        } else {
            Some(shared_context.to_string())
        },
        workspace_path: request.workspace_path.clone(),
    };

    let start = Instant::now();
    emit_stage(
        app,
        run_id,
        &PipelineStage::DirectTask,
        &StageStatus::Running,
        1,
    );

    // Mark session as live while executing
    let _ = sessions::touch_session(&request.workspace_path, session_id, None, Some("running"), None);

    if wait_if_paused(pause_flag, cancel_flag).await || is_cancelled(cancel_flag) {
        push_cancel_iteration(run, 1, Vec::new());
        return Ok(());
    }

    let result = loop {
        let dispatch_future = async {
            if settings.agent_timeout_ms == 0 {
                dispatch_agent(
                    backend,
                    model,
                    &input,
                    settings,
                    Some(session_id),
                    app,
                    run_id,
                    PipelineStage::DirectTask,
                    None,
                    None,
                )
                .await
            } else {
                match tokio::time::timeout(
                    Duration::from_millis(settings.agent_timeout_ms),
                    dispatch_agent(
                        backend,
                        model,
                        &input,
                        settings,
                        Some(session_id),
                        app,
                        run_id,
                        PipelineStage::DirectTask,
                        None,
                        None,
                    ),
                )
                .await
                {
                    Ok(inner) => inner,
                    Err(_) => Err(format!(
                        "DirectTask stage timed out after {} ms",
                        settings.agent_timeout_ms
                    )),
                }
            }
        };

        let outcome = tokio::select! {
            res = dispatch_future => Ok(res),
            interrupt = wait_for_interrupt(pause_flag, cancel_flag) => Err(interrupt),
        };

        match outcome {
            Ok(res) => break res,
            Err(RunInterrupt::Cancel) => {
                push_cancel_iteration(run, 1, Vec::new());
                return Ok(());
            }
            Err(RunInterrupt::Pause) => {
                if wait_if_paused(pause_flag, cancel_flag).await {
                    push_cancel_iteration(run, 1, Vec::new());
                    return Ok(());
                }
                emit_stage(
                    app,
                    run_id,
                    &PipelineStage::DirectTask,
                    &StageStatus::Running,
                    1,
                );
            }
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    let (status, output, error, verdict) = match result {
        Ok(dr) => (
            StageStatus::Completed,
            dr.output.raw_text,
            None,
            Some(JudgeVerdict::Complete),
        ),
        Err(e) => (StageStatus::Failed, String::new(), Some(e), None),
    };
    emit_stage_with_duration(
        app,
        run_id,
        &PipelineStage::DirectTask,
        &status,
        1,
        Some(duration_ms),
    );

    // Append stage events to event log
    let ws = &request.workspace_path;
    let seq_start = runs::next_sequence(ws, session_id, run_id).unwrap_or(1);
    let stage_start = RunEvent::StageStart {
        v: 1,
        seq: seq_start,
        ts: storage::now_rfc3339(),
        stage: PipelineStage::DirectTask,
        iteration: 1,
    };
    let _ = runs::append_event(ws, session_id, run_id, stage_start);

    let stage_end = RunEvent::StageEnd {
        v: 1,
        seq: seq_start + 1,
        ts: storage::now_rfc3339(),
        stage: PipelineStage::DirectTask,
        iteration: 1,
        status: if error.is_none() {
            StageEndStatus::Completed
        } else {
            StageEndStatus::Failed
        },
        duration_ms,
        verdict: verdict.clone(),
        input_tokens: None,
        output_tokens: None,
        estimated_cost_usd: None,
        session_pair: None,
        resumed: None,
    };
    let _ = runs::append_event(ws, session_id, run_id, stage_end);

    run.iterations.push(Iteration {
        number: 1,
        stages: vec![StageResult {
            stage: PipelineStage::DirectTask,
            status,
            output,
            duration_ms,
            error: error.clone(),
            provider_session_ref: None,
            session_pair: None,
            resumed: None,
        }],
        verdict,
        judge_reasoning: None,
    });
    if let Some(ref e) = error {
        run.status = PipelineStatus::Failed;
        run.error = Some(format!("{}", e));
    } else {
        run.status = PipelineStatus::Completed;
        run.final_verdict = Some(JudgeVerdict::Complete);
    }

    Ok(())
}
