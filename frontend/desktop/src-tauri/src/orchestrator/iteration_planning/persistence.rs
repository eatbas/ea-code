//! Persistence helpers for iteration planning.

use crate::models::{PipelineRun, PipelineStage, RunEvent, StageEndStatus};
use crate::storage::{self, runs};

/// Appends a stage_start event to the event log.
pub fn append_stage_start_event(
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
    runs::append_event(run_id, event)
}

/// Appends a stage_end event to the event log.
pub fn append_stage_end_event(
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
    status: &StageEndStatus,
    duration_ms: u64,
) -> Result<(), String> {
    let event = RunEvent::StageEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
        status: status.clone(),
        duration_ms,
        verdict: None,
    };
    runs::append_event(run_id, event)
}

/// Updates the run summary.json with current state for crash recovery.
pub fn update_run_summary(run_id: &str, run: &PipelineRun) -> Result<(), String> {
    if let Ok(mut summary) = runs::read_summary(run_id) {
        summary.status = run.status.clone().into();
        summary.current_stage = run.current_stage.clone();
        summary.current_iteration = Some(run.current_iteration);
        summary.total_iterations = run.iterations.len() as u32;
        if let Some(ref error) = run.error {
            summary.error = Some(error.clone());
        }
        let _ = runs::update_summary(run_id, &summary);
    }
    Ok(())
}
