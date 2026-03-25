//! Shared stage event persistence helpers used across planning, review, and judge stages.

use std::mem;

use crate::models::{
    Iteration, JudgeVerdict, PipelineRun, PipelineStage, PipelineStatus, RunEvent, StageEndStatus,
    StageResult,
};
use crate::storage::{self, runs, sessions};

/// Appends a stage_start event to the event log.
pub fn append_stage_start_event(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
) -> Result<(), String> {
    let event = RunEvent::StageStart {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
    };
    runs::append_event(workspace_path, session_id, run_id, event)
}

/// Appends a stage_end event to the event log.
pub fn append_stage_end_event(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
    status: &StageEndStatus,
    duration_ms: u64,
) -> Result<(), String> {
    append_stage_end_event_with_verdict(
        workspace_path,
        session_id,
        run_id,
        stage,
        iteration,
        seq,
        status,
        duration_ms,
        None,
    )
}

/// Appends a stage_end event with an optional verdict to the event log.
pub fn append_stage_end_event_with_verdict(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
    status: &StageEndStatus,
    duration_ms: u64,
    verdict: Option<JudgeVerdict>,
) -> Result<(), String> {
    let event = RunEvent::StageEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
        status: status.clone(),
        duration_ms,
        verdict,
        input_tokens: None,
        output_tokens: None,
        estimated_cost_usd: None,
        session_pair: None,
        resumed: None,
    };
    runs::append_event(workspace_path, session_id, run_id, event)
}

/// Updates the run summary.json with current state for crash recovery,
/// and touches the parent session with the latest status.
pub fn update_run_summary(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    run: &PipelineRun,
) -> Result<(), String> {
    if let Ok(mut summary) = runs::read_summary(workspace_path, session_id, run_id) {
        summary.status = run.status.clone().into();
        summary.current_stage = run.current_stage.clone();
        summary.current_iteration = Some(run.current_iteration);
        summary.total_iterations = run.iterations.len() as u32;
        if let Some(ref error) = run.error {
            summary.error = Some(error.clone());
        }
        let _ = runs::update_summary(workspace_path, session_id, run_id, &summary);
    }

    let status_str = match run.status {
        PipelineStatus::Completed => "completed",
        PipelineStatus::Failed => "failed",
        PipelineStatus::Cancelled => "cancelled",
        PipelineStatus::Running => "running",
        _ => "unknown",
    };

    let verdict_str = run.final_verdict.as_ref().map(|v| match v {
        JudgeVerdict::Complete => "COMPLETE",
        JudgeVerdict::NotComplete => "NOT COMPLETE",
    });

    if let Err(e) = sessions::touch_session(workspace_path, session_id, None, Some(status_str), verdict_str) {
        eprintln!("Warning: Failed to touch session: {e}");
    }

    Ok(())
}

/// Handles a failed stage: appends the end event, pushes the iteration as failed,
/// and updates the run summary. Returns `Ok(())` so callers can early-return.
pub fn handle_stage_failure(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iter_num: u32,
    seq: u64,
    duration_ms: u64,
    error_msg: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
) -> Result<(), String> {
    append_stage_end_event(
        workspace_path,
        session_id,
        run_id,
        stage,
        iter_num,
        seq,
        &StageEndStatus::Failed,
        duration_ms,
    )?;
    run.iterations.push(Iteration {
        number: iter_num,
        stages: mem::take(stages),
        verdict: None,
        judge_reasoning: None,
    });
    run.status = PipelineStatus::Failed;
    run.error = Some(error_msg.to_string());
    update_run_summary(workspace_path, session_id, run_id, run)?;
    Ok(())
}
