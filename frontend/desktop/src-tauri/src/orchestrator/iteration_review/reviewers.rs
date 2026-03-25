//! Parallel reviewer execution: runs 2+ reviewers in parallel, then delegates
//! to the review merger stage.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::*;
use crate::storage::runs;

use crate::orchestrator::helpers::wait_if_paused;
use crate::orchestrator::parallel_stage::{
    run_parallel_stage_tasks, stage_failed_due_to_pause, ParallelStageRun, ParallelStageSlot,
    ParallelStageTask,
};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::helpers::events;
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};

/// Runs 2+ reviewers in parallel, then runs the Review Merger stage.
/// Each reviewer gets its own system prompt with a unique output path (review_1, review_2, etc.).
#[allow(clippy::too_many_arguments)]
pub async fn run_parallel_reviewers_and_merge(
    app: &AppHandle,
    request: &PipelineRequest,
    run_id: &str,
    iter_num: u32,
    slots: &[ParallelStageSlot],
    user_prompt: &str,
    meta: &PromptMeta,
    workspace_context: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    enhanced: &str,
    run: &mut PipelineRun,
    stages_vec: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
    tracker: &crate::orchestrator::helpers::CliSessionTracker,
) -> Result<String, String> {
    // Pre-compute descriptive artifact names (e.g. review_1_claude_opus-4) so each
    // reviewer writes to a unique, identifiable file.
    let reviewer_output_kinds: Vec<String> = slots
        .iter()
        .enumerate()
        .map(|(i, slot)| {
            let model =
                crate::orchestrator::helpers::resolve_stage_model(&slot.stage, settings);
            crate::orchestrator::helpers::descriptive_artifact_name(
                "review",
                i,
                &slot.backend,
                &model,
            )
        })
        .collect();

    // Per-agent file watchdog: each reviewer gets its own abort flag.
    let review_iter_dir = runs::iteration_dir_path(
        &request.workspace_path,
        session_id,
        run_id,
        iter_num,
    )
    .unwrap_or_default();
    let per_reviewer_aborts: Vec<std::sync::Arc<std::sync::atomic::AtomicBool>> =
        reviewer_output_kinds
            .iter()
            .map(|kind| {
                let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let dir = review_iter_dir.clone();
                let slot_prefix =
                    crate::orchestrator::helpers::slot_prefix_from_output_kind(kind);
                let watcher_flag = flag.clone();
                tokio::spawn(async move {
                    crate::orchestrator::helpers::per_agent_file_watchdog(
                        dir,
                        slot_prefix,
                        watcher_flag,
                    )
                    .await;
                });
                flag
            })
            .collect();

    let review_texts = loop {
        let tasks: Vec<ParallelStageTask> = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let app = app.clone();
                let backend = slot.backend.clone();
                let task_stage = slot.stage.clone();
                let future_stage = task_stage.clone();
                let prompt = user_prompt.to_string();
                let ws = request.workspace_path.clone();
                let settings = settings.clone();
                let cf = cancel_flag.clone();
                let pf = pause_flag.clone();
                let sid = session_id.to_string();
                let rid = run_id.to_string();
                let abort = Some(per_reviewer_aborts[i].clone());

                // Look up the corresponding planner's session ref for this slot.
                // Reviewer[0] resumes Planner[0]'s session, Reviewer[1] -> Planner[1], etc.
                let reviewer_session = tracker
                    .get_ref_for_stage(&slot.stage, &slot.backend)
                    .map(|s| s.to_string());

                let output_kind = reviewer_output_kinds[i].clone();
                let output_path =
                    runs::artifact_output_path(&ws, &sid, &rid, iter_num, &output_kind).ok();
                let output_path_str = output_path.map(|p| p.to_string_lossy().to_string());

                // Build per-reviewer context with unique output path
                let reviewer_rel =
                    runs::artifact_relative_path(session_id, run_id, iter_num, &output_kind);
                let ctx = super::super::run_setup::compose_agent_context(
                    prompts::build_reviewer_system(meta, Some(&reviewer_rel)),
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
                            reviewer_session.as_deref(),
                            abort,
                        )
                        .await
                    }),
                }
            })
            .collect();

        let stage_checkpoint = stages_vec.len();
        let mut pause_retry_requested = false;
        let ws_ref = request.workspace_path.clone();
        let sid_ref = session_id.to_string();
        let successful = run_parallel_stage_tasks(
            run_id,
            iter_num,
            tasks,
            |rid, stage, iteration, seq| {
                events::append_stage_start_event(&ws_ref, &sid_ref, rid, stage, iteration, seq)
            },
            |rid, stage, iteration, seq, status, dur| {
                events::append_stage_end_event(
                    &ws_ref, &sid_ref, rid, stage, iteration, seq, status, dur,
                )
            },
            |parallel_run| {
                let ParallelStageRun {
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
                stages_vec.push(result);

                if failed {
                    None
                } else {
                    Some((
                        format!("Review from Reviewer {}", index + 1),
                        output_kind,
                        output,
                    ))
                }
            },
            &request.workspace_path,
            session_id,
        )
        .await?;

        if pause_retry_requested {
            stages_vec.truncate(stage_checkpoint);
            for kind in &reviewer_output_kinds {
                if let Ok(path) = runs::artifact_output_path(
                    &request.workspace_path,
                    session_id,
                    run_id,
                    iter_num,
                    kind,
                ) {
                    let _ = std::fs::remove_file(path);
                }
            }
            if wait_if_paused(pause_flag, cancel_flag).await {
                return Ok(String::new());
            }
            continue;
        }

        // Carry (label, output, output_kind) so the merger can file-reference correctly.
        let mut merged_review_texts: Vec<(String, String, String)> = Vec::new();
        for (title, output_kind, output) in successful {
            crate::orchestrator::helpers::emit_artifact(
                app,
                &request.workspace_path,
                session_id,
                run_id,
                &output_kind,
                &output,
                iter_num,
            );
            merged_review_texts.push((title, output, output_kind));
        }
        break merged_review_texts;
    };

    if review_texts.is_empty() {
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages_vec),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("All reviewer stages failed".to_string());
        events::update_run_summary(&request.workspace_path, session_id, run_id, run)?;
        return Ok(String::new());
    }

    // Single reviewer: return output directly without running the merger.
    if review_texts.len() == 1 {
        return Ok(review_texts.into_iter().next().unwrap().1);
    }

    // Multiple reviewers: delegate to the Review Merger stage.
    super::merger::run_review_merger_stage(
        app,
        request,
        run_id,
        iter_num,
        &review_texts,
        meta,
        workspace_context,
        settings,
        cancel_flag,
        pause_flag,
        session_id,
        enhanced,
        run,
        stages_vec,
        iter_ctx,
    )
    .await
}
