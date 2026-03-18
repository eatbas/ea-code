//! Planning stages: parallel planners (1-3) + Plan Auditor (audit or merge+audit).

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{AgentBackend, StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parsing::parse_plan_audit_output;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::*;

mod persistence;

/// A configured planner slot: (backend, pipeline stage).
struct PlannerSlot {
    backend: AgentBackend,
    stage: PipelineStage,
}

/// Collects the active planner slots from settings.
fn active_planner_slots(settings: &AppSettings) -> Vec<PlannerSlot> {
    let mut slots = Vec::new();
    if let Some(b) = &settings.planner_agent {
        slots.push(PlannerSlot { backend: b.clone(), stage: PipelineStage::Plan });
    }
    if let Some(b) = &settings.planner_2_agent {
        slots.push(PlannerSlot { backend: b.clone(), stage: PipelineStage::Plan2 });
    }
    if let Some(b) = &settings.planner_3_agent {
        slots.push(PlannerSlot { backend: b.clone(), stage: PipelineStage::Plan3 });
    }
    slots
}

/// Skips all planning-related stages and returns Ok.
fn skip_planning(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    reason: &str,
    stages: &mut Vec<StageResult>,
) {
    stages.push(execute_skipped_stage(app, run_id, iter_num, PipelineStage::Plan, reason));
    stages.push(execute_skipped_stage(app, run_id, iter_num, PipelineStage::PlanAudit, reason));
}

/// Planning stages: parallel planners (1-3) + Plan Auditor.
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
        skip_planning(app, run_id, iter_num, "No planners configured; skipping planning.", stages);
        return Ok(());
    }

    // Plan Auditor must be configured when planners are active.
    if settings.plan_auditor_agent.is_none() {
        skip_planning(
            app, run_id, iter_num,
            "Plan Auditor must be configured when planners are active; skipping planning.",
            stages,
        );
        return Ok(());
    }

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Run planners (1 sequential, 2-3 parallel) ---
    let planner_user_prompt = prompts::build_planner_user(
        &request.prompt,
        enhanced,
        iter_ctx.selected_plan(),
        judge_feedback,
    );
    let planner_context = super::run_setup::compose_agent_context(
        prompts::build_planner_system(meta),
        workspace_context,
    );

    let plan_outputs = if planner_slots.len() == 1 {
        run_single_planner(
            app, run_id, iter_num, &planner_slots[0], &planner_user_prompt,
            &planner_context, &request.workspace_path, settings, cancel_flag,
            session_id, stages,
        ).await?
    } else {
        run_parallel_planners(
            app, run_id, iter_num, &planner_slots, &planner_user_prompt,
            &planner_context, &request.workspace_path, settings, cancel_flag,
            session_id, stages,
        ).await?
    };

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
        persistence::update_run_summary(run_id, run)?;
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

    // --- Plan Audit stage (audit if 1 plan, merge+audit if 2-3 plans) ---
    let auditor_backend = settings.plan_auditor_agent.as_ref()
        .unwrap_or(&AgentBackend::Claude);

    let (system_prompt, user_prompt) = if plan_outputs.len() == 1 {
        (
            prompts::build_plan_auditor_system(meta),
            prompts::build_plan_auditor_user(
                &request.prompt, enhanced, &plan_outputs[0].1,
                None, None, judge_feedback,
            ),
        )
    } else {
        (
            prompts::build_plan_auditor_merge_system(meta),
            prompts::build_plan_auditor_merge_user(
                &request.prompt, enhanced, &plan_outputs,
                iter_ctx.selected_plan(), judge_feedback,
            ),
        )
    };

    let pa_seq = runs::next_sequence(run_id).unwrap_or(1);
    persistence::append_stage_start_event(run_id, &PipelineStage::PlanAudit, iter_num, pa_seq)?;

    let pa_output_path = runs::artifact_output_path(run_id, iter_num, "plan_audit").ok();
    let pa_output_path_str = pa_output_path.as_ref().map(|p| p.to_string_lossy().to_string());

    let pa_r = execute_agent_stage(
        app, run_id, iter_num, PipelineStage::PlanAudit, auditor_backend,
        &AgentInput {
            prompt: user_prompt,
            context: Some(super::run_setup::compose_agent_context(system_prompt, workspace_context)),
            workspace_path: request.workspace_path.clone(),
        },
        settings, cancel_flag, Some(session_id),
        pa_output_path_str.as_deref(),
    ).await;
    let pa_out = pa_r.output.clone();
    let pa_duration = pa_r.duration_ms;

    if pa_r.status == StageStatus::Failed {
        let err = pa_r.error.clone().unwrap_or_else(|| "Plan Auditor stage failed".into());
        stages.push(pa_r);
        persistence::append_stage_end_event(
            run_id, &PipelineStage::PlanAudit, iter_num, pa_seq + 1,
            &StageEndStatus::Failed, pa_duration,
        )?;
        run.iterations.push(Iteration {
            number: iter_num, stages: mem::take(stages),
            verdict: None, judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some(err);
        persistence::update_run_summary(run_id, run)?;
        return Ok(());
    }
    stages.push(pa_r);
    persistence::append_stage_end_event_with_audit(
        run_id, &PipelineStage::PlanAudit, iter_num, pa_seq + 1,
        &StageEndStatus::Completed, pa_duration, &pa_out,
    )?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // Use the primary planner output as fallback for audit parsing.
    let fallback_plan = &plan_outputs[0].1;
    let parsed = parse_plan_audit_output(&pa_out, fallback_plan);
    iter_ctx.audit_verdict = Some(parsed.verdict);
    iter_ctx.audit_reasoning = if parsed.reasoning.trim().is_empty() { None } else { Some(parsed.reasoning) };
    iter_ctx.audited_plan = Some(parsed.improved_plan.clone());

    crate::orchestrator::helpers::emit_artifact(app, run_id, "plan_audit", &parsed.improved_plan, iter_num);

    Ok(())
}

/// Runs a single planner and returns its output. Returns empty vec on failure.
#[allow(clippy::too_many_arguments)]
async fn run_single_planner(
    app: &AppHandle, run_id: &str, iter_num: u32,
    slot: &PlannerSlot, user_prompt: &str, context: &str,
    workspace_path: &str, settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>, session_id: &str,
    stages: &mut Vec<StageResult>,
) -> Result<Vec<(String, String)>, String> {
    let seq = runs::next_sequence(run_id).unwrap_or(1);
    persistence::append_stage_start_event(run_id, &slot.stage, iter_num, seq)?;

    let output_path = runs::artifact_output_path(run_id, iter_num, "plan").ok();
    let output_path_str = output_path.as_ref().map(|p| p.to_string_lossy().to_string());

    let r = execute_agent_stage(
        app, run_id, iter_num, slot.stage.clone(), &slot.backend,
        &AgentInput {
            prompt: user_prompt.to_string(),
            context: Some(context.to_string()),
            workspace_path: workspace_path.to_string(),
        },
        settings, cancel_flag, Some(session_id),
        output_path_str.as_deref(),
    ).await;
    let out = r.output.clone();
    let dur = r.duration_ms;
    let failed = r.status == StageStatus::Failed;

    if !failed {
        crate::orchestrator::helpers::emit_artifact(app, run_id, "plan", &out, iter_num);
    }

    let status = if failed { &StageEndStatus::Failed } else { &StageEndStatus::Completed };
    persistence::append_stage_end_event(run_id, &slot.stage, iter_num, seq + 1, status, dur)?;
    stages.push(r);

    if failed {
        Ok(vec![])
    } else {
        Ok(vec![("Proposed Plan".to_string(), out)])
    }
}

/// Runs 2-3 planners in parallel via tokio::join! and collects successful outputs.
#[allow(clippy::too_many_arguments)]
async fn run_parallel_planners(
    app: &AppHandle, run_id: &str, iter_num: u32,
    slots: &[PlannerSlot], user_prompt: &str, context: &str,
    workspace_path: &str, settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>, session_id: &str,
    stages: &mut Vec<StageResult>,
) -> Result<Vec<(String, String)>, String> {
    let base_seq = runs::next_sequence(run_id).unwrap_or(1);
    let mut end_sequences = Vec::with_capacity(slots.len());
    for (index, slot) in slots.iter().enumerate() {
        let start_seq = base_seq + (index as u64 * 2);
        persistence::append_stage_start_event(run_id, &slot.stage, iter_num, start_seq)?;
        end_sequences.push(start_seq + 1);
    }

    // Build futures for each planner slot.
    let futures: Vec<_> = slots.iter().enumerate().map(|(i, slot)| {
        let app = app.clone();
        let backend = slot.backend.clone();
        let stage = slot.stage.clone();
        let end_seq = end_sequences[i];
        let prompt = user_prompt.to_string();
        let ctx = context.to_string();
        let ws = workspace_path.to_string();
        let settings = settings.clone();
        let cf = cancel_flag.clone();
        let sid = session_id.to_string();
        let rid = run_id.to_string();

        let output_kind = format!("plan_{}", i + 1);
        let output_path = runs::artifact_output_path(&rid, iter_num, &output_kind).ok();
        let output_path_str = output_path.map(|p| p.to_string_lossy().to_string());

        async move {
            let r = execute_agent_stage(
                &app, &rid, iter_num, stage.clone(), &backend,
                &AgentInput {
                    prompt,
                    context: Some(ctx),
                    workspace_path: ws,
                },
                &settings, &cf, Some(&sid),
                output_path_str.as_deref(),
            ).await;
            (i, stage, end_seq, r)
        }
    }).collect();

    // Execute all planners concurrently.
    let results = futures::future::join_all(futures).await;

    let mut plan_outputs = Vec::new();
    for (i, stage, end_seq, r) in results {
        let out = r.output.clone();
        let failed = r.status == StageStatus::Failed;
        let status = if failed {
            StageEndStatus::Failed
        } else {
            StageEndStatus::Completed
        };
        persistence::append_stage_end_event(
            run_id,
            &stage,
            iter_num,
            end_seq,
            &status,
            r.duration_ms,
        )?;

        if !failed {
            let label = format!("Plan from Planner {}", i + 1);
            crate::orchestrator::helpers::emit_artifact(app, run_id, &format!("plan_{}", i + 1), &out, iter_num);
            plan_outputs.push((label, out));
        }
        stages.push(r);
    }

    Ok(plan_outputs)
}
