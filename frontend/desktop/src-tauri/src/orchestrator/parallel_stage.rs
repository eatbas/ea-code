//! Shared helpers for running parallel pipeline stage groups.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::models::{PipelineStage, StageEndStatus, StageResult, StageStatus};
use crate::storage::runs;

/// Delay between launching each parallel task to reduce pressure on the sidecar.
/// The first task starts immediately; subsequent tasks are staggered by this interval.
const PARALLEL_STAGGER_MS: u64 = 300;

/// A configured parallel stage slot.
#[derive(Clone)]
pub struct ParallelStageSlot {
    pub backend: crate::models::AgentBackend,
    pub stage: PipelineStage,
}

/// An executable parallel stage task.
pub struct ParallelStageTask {
    pub stage: PipelineStage,
    pub output_kind: String,
    pub future: Pin<Box<dyn Future<Output = StageResult> + Send>>,
}

/// Result of a parallel stage task after the shared event bookkeeping runs.
pub struct ParallelStageRun {
    pub index: usize,
    pub output_kind: String,
    pub result: StageResult,
}

/// Runs a batch of parallel stage tasks with shared start/end event handling.
///
/// The caller owns the task futures and the success handling logic; this helper
/// only centralises sequence allocation, event emission, and ordered result
/// folding so planner/reviewer orchestration stays aligned.
pub async fn run_parallel_stage_tasks<T, OnResult>(
    run_id: &str,
    iteration: u32,
    tasks: Vec<ParallelStageTask>,
    append_stage_start_event: impl Fn(&str, &PipelineStage, u32, u64) -> Result<(), String>,
    append_stage_end_event: impl Fn(
        &str,
        &PipelineStage,
        u32,
        u64,
        &StageEndStatus,
        u64,
    ) -> Result<(), String>,
    mut on_result: OnResult,
    workspace_path: &str,
    session_id: &str,
) -> Result<Vec<T>, String>
where
    OnResult: FnMut(ParallelStageRun) -> Option<T>,
{
    if tasks.is_empty() {
        return Ok(Vec::new());
    }

    let base_seq = runs::next_sequence(workspace_path, session_id, run_id).unwrap_or(1);
    let mut end_sequences = Vec::with_capacity(tasks.len());

    for (index, task) in tasks.iter().enumerate() {
        let start_seq = base_seq + (index as u64 * 2);
        append_stage_start_event(run_id, &task.stage, iteration, start_seq)?;
        end_sequences.push(start_seq + 1);
    }

    let stages: Vec<PipelineStage> = tasks.iter().map(|task| task.stage.clone()).collect();
    let output_kinds: Vec<String> = tasks.iter().map(|task| task.output_kind.clone()).collect();

    // Stagger task launches so they don't all hit the sidecar simultaneously,
    // which can cause the single-worker uvicorn process to crash under load.
    let results = futures::future::join_all(tasks.into_iter().enumerate().map(|(i, task)| {
        let delay = Duration::from_millis(PARALLEL_STAGGER_MS * i as u64);
        async move {
            if i > 0 {
                tokio::time::sleep(delay).await;
            }
            task.future.await
        }
    }))
    .await;

    let mut successful = Vec::new();
    for (index, ((stage, output_kind), result)) in stages
        .into_iter()
        .zip(output_kinds.into_iter())
        .zip(results.into_iter())
        .enumerate()
    {
        let end_seq = end_sequences[index];
        let status = if result.status == StageStatus::Failed {
            StageEndStatus::Failed
        } else {
            StageEndStatus::Completed
        };

        append_stage_end_event(
            run_id,
            &stage,
            iteration,
            end_seq,
            &status,
            result.duration_ms,
        )?;

        let parallel_run = ParallelStageRun {
            index,
            output_kind,
            result,
        };

        if let Some(output) = on_result(parallel_run) {
            successful.push(output);
        }
    }

    Ok(successful)
}
