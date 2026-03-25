//! Planner execution: runs 1-N planners in parallel.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::*;
use crate::storage::runs;

use crate::orchestrator::helpers::wait_if_paused;
use crate::orchestrator::parallel_stage::{
    run_parallel_stage_tasks, stage_failed_due_to_pause, ParallelStageSlot, ParallelStageTask,
};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};

use crate::orchestrator::helpers::events;

/// Runs 1+ planners in parallel via tokio tasks and collects successful outputs.
/// Each planner gets its own system prompt with a unique output path (plan_1, plan_2, etc.).
#[allow(clippy::too_many_arguments)]
pub async fn run_parallel_planners(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    slots: &[ParallelStageSlot],
    user_prompt: &str,
    meta: &PromptMeta,
    workspace_context: &str,
    workspace_path: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    stages: &mut Vec<StageResult>,
    tracker: &crate::orchestrator::helpers::CliSessionTracker,
) -> Result<Vec<(String, String, String)>, String> {
    // Pre-compute descriptive artifact names (e.g. plan_1_claude_opus-4) so each
    // planner writes to a unique, identifiable file. We store these so the pause
    // cleanup and plan audit can reference the exact filenames.
    let output_kinds: Vec<String> = slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let model = crate::orchestrator::helpers::resolve_stage_model(&slot.stage, settings);
            crate::orchestrator::helpers::descriptive_artifact_name("plan", i, &slot.backend, &model)
        })
        .collect();

    // Per-agent file watchdog: each planner gets its own abort flag.
    // When planner N's file appears (plan_N*.md), only that agent is aborted.
    let iter_dir = runs::iteration_dir_path(workspace_path, session_id, run_id, iter_num)
        .unwrap_or_default();
    let per_agent_aborts: Vec<std::sync::Arc<std::sync::atomic::AtomicBool>> = output_kinds
        .iter()
        .map(|kind| {
            let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let dir = iter_dir.clone();
            let slot_prefix = crate::orchestrator::helpers::slot_prefix_from_output_kind(kind);
            let watcher_flag = flag.clone();
            tokio::spawn(async move {
                crate::orchestrator::helpers::per_agent_file_watchdog(dir, slot_prefix, watcher_flag).await;
            });
            flag
        })
        .collect();

    loop {
        let tasks: Vec<ParallelStageTask> = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let app = app.clone();
                let backend = slot.backend.clone();
                let task_stage = slot.stage.clone();
                let future_stage = task_stage.clone();
                let prompt = user_prompt.to_string();
                let ws = workspace_path.to_string();
                let settings = settings.clone();
                let cf = cancel_flag.clone();
                let pf = pause_flag.clone();
                let sid = session_id.to_string();
                let rid = run_id.to_string();
                let abort = Some(per_agent_aborts[i].clone());

                // Look up a prior session ref for this planner slot (e.g. from
                // a previous iteration). On first iteration this is None.
                let planner_session = tracker
                    .get_ref_for_stage(&slot.stage, &slot.backend)
                    .map(|s| s.to_string());

                let output_kind = output_kinds[i].clone();
                let output_path = runs::artifact_output_path(&ws, &sid, &rid, iter_num, &output_kind).ok();
                let output_path_str = output_path.map(|p| p.to_string_lossy().to_string());

                // Build per-planner context with unique output path
                let planner_rel = runs::artifact_relative_path(session_id, run_id, iter_num, &output_kind);
                let ctx = crate::orchestrator::run_setup::compose_agent_context(
                    prompts::build_planner_system(meta, Some(&planner_rel)),
                    workspace_context,
                );

                ParallelStageTask {
                    stage: task_stage,
                    output_kind,
                    future: Box::pin(async move {
                        execute_agent_stage(
                            &app,
                            &rid,
                            iter_num,
                            future_stage,
                            &backend,
                            &AgentInput {
                                prompt,
                                context: Some(ctx),
                                workspace_path: ws,
                            },
                            &settings,
                            &cf,
                            &pf,
                            PauseHandling::ReturnPausedError,
                            Some(&sid),
                            output_path_str.as_deref(),
                            planner_session.as_deref(),
                            abort,
                        )
                        .await
                    }),
                }
            })
            .collect();

        let stage_checkpoint = stages.len();
        let mut pause_retry_requested = false;
        let ws_ref = workspace_path.to_string();
        let sid_ref = session_id.to_string();
        let successful = run_parallel_stage_tasks(
            run_id,
            iter_num,
            tasks,
            |rid, stage, iteration, seq| events::append_stage_start_event(&ws_ref, &sid_ref, rid, stage, iteration, seq),
            |rid, stage, iteration, seq, status, dur| events::append_stage_end_event(&ws_ref, &sid_ref, rid, stage, iteration, seq, status, dur),
            |parallel_run| {
                let crate::orchestrator::parallel_stage::ParallelStageRun {
                    index,
                    output_kind,
                    result,
                    ..
                } = parallel_run;
                let paused = stage_failed_due_to_pause(&result);
                let failed = result.status == StageStatus::Failed;
                let output = result.output.clone();
                if paused {
                    pause_retry_requested = true;
                }
                stages.push(result);

                if failed {
                    None
                } else {
                    Some((index, output_kind, output))
                }
            },
            workspace_path,
            session_id,
        )
        .await?;

        if pause_retry_requested {
            stages.truncate(stage_checkpoint);
            for kind in &output_kinds {
                if let Ok(path) = runs::artifact_output_path(workspace_path, session_id, run_id, iter_num, kind) {
                    let _ = std::fs::remove_file(path);
                }
            }
            if wait_if_paused(pause_flag, cancel_flag).await {
                return Ok(Vec::new());
            }
            continue;
        }

        // Return (label, content, output_kind) so the plan audit stage can
        // reference the exact filenames that were written.
        let mut plan_outputs: Vec<(String, String, String)> = Vec::new();
        for (index, output_kind, output) in successful {
            crate::orchestrator::helpers::emit_artifact(
                app,
                workspace_path,
                session_id,
                run_id,
                &output_kind,
                &output,
                iter_num,
            );
            let cleaned = crate::orchestrator::parsing::clean_plan_output(&output);
            plan_outputs.push((format!("Plan from Planner {}", index + 1), cleaned, output_kind));
        }
        return Ok(plan_outputs);
    }
}
