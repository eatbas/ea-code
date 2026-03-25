//! Planning stages: parallel planners (1-N) + Plan Auditor (audit or merge+audit).

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{AgentBackend, StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parallel_stage::{
    run_parallel_stage_tasks, ParallelStageSlot, ParallelStageTask,
};
use crate::orchestrator::parsing::parse_plan_audit_output;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::*;

mod persistence;

/// Collects the active planner slots from settings.
fn active_planner_slots(settings: &AppSettings) -> Vec<ParallelStageSlot> {
    let mut slots = Vec::new();
    if let Some(b) = &settings.planner_agent {
        slots.push(ParallelStageSlot {
            backend: b.clone(),
            stage: PipelineStage::Plan,
        });
    }
    for (i, slot) in settings.extra_planners.iter().enumerate() {
        if let Some(b) = &slot.agent {
            slots.push(ParallelStageSlot {
                backend: b.clone(),
                stage: PipelineStage::ExtraPlan(i as u8),
            });
        }
    }
    slots
}

fn stage_failed_due_to_pause(result: &StageResult) -> bool {
    if result.status != StageStatus::Failed {
        return false;
    }
    result
        .error
        .as_ref()
        .map(|err| err.to_ascii_lowercase().contains("paused by user"))
        .unwrap_or(false)
}

/// Skips all planning-related stages and returns Ok.
fn skip_planning(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    reason: &str,
    stages: &mut Vec<StageResult>,
) {
    stages.push(execute_skipped_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::Plan,
        reason,
    ));
    stages.push(execute_skipped_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::PlanAudit,
        reason,
    ));
}

/// Planning stages: parallel planners (1-N) + Plan Auditor.
#[allow(clippy::too_many_arguments)]
pub async fn run_planning_stages(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    meta: &PromptMeta,
    enhanced: &str,
    judge_feedback: Option<&str>,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
    workspace_context: &str,
) -> Result<(), String> {
    // Budget mode or no_plan → skip planning entirely.
    if request.no_plan || settings.budget_mode {
        let reason = if settings.budget_mode {
            "Planning stages skipped (Budget Mode)."
        } else {
            "Planning stages skipped by user request (No Plan mode)."
        };
        skip_planning(app, run_id, iter_num, reason, stages);
        return Ok(());
    }

    let planner_slots = active_planner_slots(settings);
    if planner_slots.is_empty() {
        skip_planning(
            app,
            run_id,
            iter_num,
            "No planners configured; skipping planning.",
            stages,
        );
        return Ok(());
    }

    // Plan Auditor must be configured when planners are active.
    if settings.plan_auditor_agent.is_none() {
        skip_planning(
            app,
            run_id,
            iter_num,
            "Plan Auditor must be configured when planners are active; skipping planning.",
            stages,
        );
        return Ok(());
    }

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Run planners (1 sequential, 2+ parallel) ---
    let ws = &request.workspace_path;
    let enhanced_ref = crate::orchestrator::helpers::artifact_file_path(
        ws, session_id, run_id, iter_num, "enhanced_prompt",
    )
    .map(|p| crate::orchestrator::helpers::file_ref(&p))
    .unwrap_or_else(|| enhanced.to_string());

    let plan_ref = iter_ctx.selected_plan().and_then(|_| {
        crate::orchestrator::helpers::artifact_file_path(ws, session_id, run_id, iter_num, "plan_audit")
            .map(|p| crate::orchestrator::helpers::file_ref(&p))
    });

    let planner_user_prompt = prompts::build_planner_user(
        &request.prompt,
        &enhanced_ref,
        plan_ref.as_deref(),
        judge_feedback,
    );
    let planner_output_rel = runs::artifact_relative_path(session_id, run_id, iter_num, "plan");
    let planner_context = super::run_setup::compose_agent_context(
        prompts::build_planner_system(meta, Some(&planner_output_rel)),
        workspace_context,
    );

    let plan_outputs = if planner_slots.len() == 1 {
        run_single_planner(
            app,
            run_id,
            iter_num,
            &planner_slots[0],
            &planner_user_prompt,
            &planner_context,
            &request.workspace_path,
            settings,
            cancel_flag,
            pause_flag,
            session_id,
            stages,
        )
        .await?
    } else {
        run_parallel_planners(
            app,
            run_id,
            iter_num,
            &planner_slots,
            &planner_user_prompt,
            &planner_context,
            &request.workspace_path,
            settings,
            cancel_flag,
            pause_flag,
            session_id,
            stages,
        )
        .await?
    };

    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    if plan_outputs.is_empty() {
        // All planners failed; stages already pushed, mark run as failed.
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("All planner stages failed".to_string());
        persistence::update_run_summary(ws, session_id, run_id, run)?;
        return Ok(());
    }

    // Store the primary plan output for backward compatibility.
    iter_ctx.planner_plan = Some(plan_outputs[0].1.clone());

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Plan Audit stage (audit if 1 plan, merge+audit if 2+ plans) ---
    let auditor_backend = settings
        .plan_auditor_agent
        .as_ref()
        .unwrap_or(&AgentBackend::Claude);

    // Build file references for the plan audit prompt.
    let audit_enhanced_ref = crate::orchestrator::helpers::artifact_file_path(
        ws, session_id, run_id, iter_num, "enhanced_prompt",
    )
    .map(|p| crate::orchestrator::helpers::file_ref(&p))
    .unwrap_or_else(|| enhanced.to_string());

    let pa_output_rel = runs::artifact_relative_path(session_id, run_id, iter_num, "plan_audit");
    let (system_prompt, user_prompt) = if plan_outputs.len() == 1 {
        // Single plan — file-ref if the artifact was written.
        let plan_kind = "plan";
        let plan_ref = crate::orchestrator::helpers::artifact_file_path(
            ws, session_id, run_id, iter_num, plan_kind,
        )
        .map(|p| crate::orchestrator::helpers::file_ref(&p))
        .unwrap_or_else(|| plan_outputs[0].1.clone());

        (
            prompts::build_plan_auditor_system(meta, Some(&pa_output_rel)),
            prompts::build_plan_auditor_user(
                &request.prompt,
                &audit_enhanced_ref,
                &plan_ref,
                None,
                None,
                judge_feedback,
            ),
        )
    } else {
        // Multiple plans — file-ref each one.
        let plan_refs: Vec<(String, String)> = plan_outputs
            .iter()
            .enumerate()
            .map(|(i, (label, content))| {
                let kind = format!("plan_{}", i + 1);
                let ref_text = crate::orchestrator::helpers::artifact_file_path(
                    ws, session_id, run_id, iter_num, &kind,
                )
                .map(|p| crate::orchestrator::helpers::file_ref(&p))
                .unwrap_or_else(|| content.clone());
                (label.clone(), ref_text)
            })
            .collect();

        let prev_plan_ref = iter_ctx.selected_plan().and_then(|_| {
            crate::orchestrator::helpers::artifact_file_path(ws, session_id, run_id, iter_num, "plan_audit")
                .map(|p| crate::orchestrator::helpers::file_ref(&p))
        });

        (
            prompts::build_plan_auditor_merge_system(meta, Some(&pa_output_rel)),
            prompts::build_plan_auditor_merge_user(
                &request.prompt,
                &audit_enhanced_ref,
                &plan_refs,
                prev_plan_ref.as_deref(),
                judge_feedback,
            ),
        )
    };

    let pa_seq = runs::next_sequence(ws, session_id, run_id).unwrap_or(1);
    persistence::append_stage_start_event(ws, session_id, run_id, &PipelineStage::PlanAudit, iter_num, pa_seq)?;

    let pa_output_path = runs::artifact_output_path(ws, session_id, run_id, iter_num, "plan_audit").ok();
    let pa_output_path_str = pa_output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let pa_input = AgentInput {
        prompt: user_prompt,
        context: Some(super::run_setup::compose_agent_context(
            system_prompt,
            workspace_context,
        )),
        workspace_path: request.workspace_path.clone(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(ws, session_id, run_id, "plan_audit", &pa_input, iter_num);

    let pa_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::PlanAudit,
        auditor_backend,
        &pa_input,
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        pa_output_path_str.as_deref(),
        None,
    )
    .await;
    let pa_out = pa_r.output.clone();
    let pa_duration = pa_r.duration_ms;

    if pa_r.status == StageStatus::Failed {
        let err = pa_r
            .error
            .clone()
            .unwrap_or_else(|| "Plan Auditor stage failed".into());
        stages.push(pa_r);
        persistence::append_stage_end_event(
            ws,
            session_id,
            run_id,
            &PipelineStage::PlanAudit,
            iter_num,
            pa_seq + 1,
            &StageEndStatus::Failed,
            pa_duration,
        )?;
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some(err);
        persistence::update_run_summary(ws, session_id, run_id, run)?;
        return Ok(());
    }
    stages.push(pa_r);
    persistence::append_stage_end_event(
        ws,
        session_id,
        run_id,
        &PipelineStage::PlanAudit,
        iter_num,
        pa_seq + 1,
        &StageEndStatus::Completed,
        pa_duration,
    )?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // Use the primary planner output as fallback for audit parsing.
    let fallback_plan = &plan_outputs[0].1;
    let parsed = parse_plan_audit_output(&pa_out, fallback_plan);
    iter_ctx.audited_plan = Some(parsed.improved_plan.clone());

    crate::orchestrator::helpers::emit_artifact(
        app,
        ws,
        session_id,
        run_id,
        "plan_audit",
        &parsed.improved_plan,
        iter_num,
    );

    Ok(())
}

/// Runs a single planner and returns its output. Returns empty vec on failure.
#[allow(clippy::too_many_arguments)]
async fn run_single_planner(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    slot: &ParallelStageSlot,
    user_prompt: &str,
    context: &str,
    workspace_path: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    stages: &mut Vec<StageResult>,
) -> Result<Vec<(String, String)>, String> {
    let seq = runs::next_sequence(workspace_path, session_id, run_id).unwrap_or(1);
    persistence::append_stage_start_event(workspace_path, session_id, run_id, &slot.stage, iter_num, seq)?;

    let output_path = runs::artifact_output_path(workspace_path, session_id, run_id, iter_num, "plan").ok();
    let output_path_str = output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let input = AgentInput {
        prompt: user_prompt.to_string(),
        context: Some(context.to_string()),
        workspace_path: workspace_path.to_string(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(workspace_path, session_id, run_id, "plan", &input, iter_num);

    let r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        slot.stage.clone(),
        &slot.backend,
        &input,
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        output_path_str.as_deref(),
        None,
    )
    .await;
    let out = r.output.clone();
    let dur = r.duration_ms;
    let failed = r.status == StageStatus::Failed;

    if !failed {
        crate::orchestrator::helpers::emit_artifact(app, workspace_path, session_id, run_id, "plan", &out, iter_num);
    }

    let status = if failed {
        &StageEndStatus::Failed
    } else {
        &StageEndStatus::Completed
    };
    persistence::append_stage_end_event(workspace_path, session_id, run_id, &slot.stage, iter_num, seq + 1, status, dur)?;
    stages.push(r);

    if failed {
        Ok(vec![])
    } else {
        let cleaned = crate::orchestrator::parsing::clean_plan_output(&out);
        Ok(vec![("Proposed Plan".to_string(), cleaned)])
    }
}

/// Runs 2+ planners in parallel via tokio::join! and collects successful outputs.
#[allow(clippy::too_many_arguments)]
async fn run_parallel_planners(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    slots: &[ParallelStageSlot],
    user_prompt: &str,
    context: &str,
    workspace_path: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    stages: &mut Vec<StageResult>,
) -> Result<Vec<(String, String)>, String> {
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
                let ctx = context.to_string();
                let ws = workspace_path.to_string();
                let settings = settings.clone();
                let cf = cancel_flag.clone();
                let pf = pause_flag.clone();
                let sid = session_id.to_string();
                let rid = run_id.to_string();

                let output_kind = format!("plan_{}", i + 1);
                let output_path = runs::artifact_output_path(&ws, &sid, &rid, iter_num, &output_kind).ok();
                let output_path_str = output_path.map(|p| p.to_string_lossy().to_string());

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
                            None,
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
            |rid, stage, iteration, seq| persistence::append_stage_start_event(&ws_ref, &sid_ref, rid, stage, iteration, seq),
            |rid, stage, iteration, seq, status, dur| persistence::append_stage_end_event(&ws_ref, &sid_ref, rid, stage, iteration, seq, status, dur),
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
            for idx in 0..slots.len() {
                let kind = format!("plan_{}", idx + 1);
                if let Ok(path) = runs::artifact_output_path(workspace_path, session_id, run_id, iter_num, &kind) {
                    let _ = std::fs::remove_file(path);
                }
            }
            if wait_if_paused(pause_flag, cancel_flag).await {
                return Ok(Vec::new());
            }
            continue;
        }

        let mut plan_outputs: Vec<(String, String)> = Vec::new();
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
            plan_outputs.push((format!("Plan from Planner {}", index + 1), cleaned));
        }
        return Ok(plan_outputs);
    }
}
