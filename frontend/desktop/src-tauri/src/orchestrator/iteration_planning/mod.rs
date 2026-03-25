//! Planning stages: parallel planners (1-N) + Plan Auditor (audit or merge+audit).

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{AgentBackend, StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parallel_stage::ParallelStageSlot;
use crate::orchestrator::parsing::parse_plan_audit_output;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::*;

mod planners;

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
    tracker: &crate::orchestrator::helpers::CliSessionTracker,
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
    let enhanced_ref = crate::orchestrator::helpers::file_ref_or_inline(
        ws, session_id, run_id, iter_num, "enhanced_prompt", enhanced,
    );

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

    let plan_outputs = planners::run_parallel_planners(
        app,
        run_id,
        iter_num,
        &planner_slots,
        &planner_user_prompt,
        meta,
        workspace_context,
        &request.workspace_path,
        settings,
        cancel_flag,
        pause_flag,
        session_id,
        stages,
        tracker,
    )
    .await?;

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
        crate::orchestrator::helpers::events::update_run_summary(ws, session_id, run_id, run)?;
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
    let audit_enhanced_ref = crate::orchestrator::helpers::file_ref_or_inline(
        ws, session_id, run_id, iter_num, "enhanced_prompt", enhanced,
    );

    let pa_output_rel = runs::artifact_relative_path(session_id, run_id, iter_num, "plan_audit");
    let (system_prompt, user_prompt) = if plan_outputs.len() == 1 {
        // Single plan — file-ref using the actual output_kind from the planner.
        let (_, content, ref output_kind) = &plan_outputs[0];
        let plan_ref = crate::orchestrator::helpers::file_ref_or_inline(
            ws, session_id, run_id, iter_num, output_kind, content,
        );

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
        // Multiple plans — file-ref each one using the stored output_kind.
        let plan_refs: Vec<(String, String)> = plan_outputs
            .iter()
            .map(|(label, content, output_kind)| {
                let ref_text = crate::orchestrator::helpers::file_ref_or_inline(
                    ws, session_id, run_id, iter_num, output_kind, content,
                );
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
    crate::orchestrator::helpers::events::append_stage_start_event(ws, session_id, run_id, &PipelineStage::PlanAudit, iter_num, pa_seq)?;

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
        crate::orchestrator::helpers::handle_stage_failure(
            ws, session_id, run_id,
            &PipelineStage::PlanAudit, iter_num, pa_seq + 1,
            pa_duration, &err, run, stages,
        )?;
        return Ok(());
    }
    stages.push(pa_r);
    crate::orchestrator::helpers::events::append_stage_end_event(
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
