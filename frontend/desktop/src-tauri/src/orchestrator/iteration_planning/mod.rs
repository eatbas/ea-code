//! Planning stages (Plan + Plan Audit) extracted from iteration.rs.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parsing::parse_plan_audit_output;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::*;

mod persistence;

/// Planning stages: Plan + Plan Audit.
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
    let planning_enabled = !request.no_plan
        && settings.planner_agent.is_some()
        && settings.plan_auditor_agent.is_some();
    if !planning_enabled {
        let skip = if request.no_plan {
            "Planning stages skipped by user request (No Plan mode)."
        } else {
            "Planner and Plan Auditor must both be selected; skipping planning stages."
        };
        stages.push(execute_skipped_stage(
            app,
            run_id,
            iter_num,
            PipelineStage::Plan,
            skip,
        ));
        stages.push(execute_skipped_stage(
            app,
            run_id,
            iter_num,
            PipelineStage::PlanAudit,
            skip,
        ));
        return Ok(());
    }

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Plan stage ---
    let plan_seq = runs::next_sequence(run_id).unwrap_or(1);
    persistence::append_stage_start_event(run_id, &PipelineStage::Plan, iter_num, plan_seq)?;

    let plan_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::Plan,
        settings
            .planner_agent
            .as_ref()
            .unwrap_or(&crate::models::AgentBackend::Claude),
        &AgentInput {
            prompt: prompts::build_planner_user(
                &request.prompt,
                enhanced,
                iter_ctx.selected_plan(),
                judge_feedback,
            ),
            context: Some(super::run_setup::compose_agent_context(
                prompts::build_planner_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        Some(session_id),
    )
    .await;
    let plan_out = plan_r.output.clone();
    let plan_duration = plan_r.duration_ms;
    iter_ctx.planner_plan = Some(plan_out.clone());

    if plan_r.status != StageStatus::Failed {
        crate::orchestrator::helpers::emit_artifact(app, run_id, "plan", &plan_out, iter_num);
    }

    if plan_r.status == StageStatus::Failed {
        stages.push(plan_r);
        persistence::append_stage_end_event(
            run_id,
            &PipelineStage::Plan,
            iter_num,
            plan_seq + 1,
            &StageEndStatus::Failed,
            plan_duration,
        )?;
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("Planner stage failed".to_string());
        persistence::update_run_summary(run_id, run)?;
        return Ok(());
    }
    stages.push(plan_r);
    persistence::append_stage_end_event(
        run_id,
        &PipelineStage::Plan,
        iter_num,
        plan_seq + 1,
        &StageEndStatus::Completed,
        plan_duration,
    )?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Plan Audit stage ---
    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    let pa_seq = runs::next_sequence(run_id).unwrap_or(1);
    persistence::append_stage_start_event(run_id, &PipelineStage::PlanAudit, iter_num, pa_seq)?;

    let pa_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::PlanAudit,
        settings
            .plan_auditor_agent
            .as_ref()
            .unwrap_or(&crate::models::AgentBackend::Claude),
        &AgentInput {
            prompt: prompts::build_plan_auditor_user(
                &request.prompt,
                enhanced,
                &plan_out,
                None,
                None,
                judge_feedback,
            ),
            context: Some(super::run_setup::compose_agent_context(
                prompts::build_plan_auditor_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        Some(session_id),
    )
    .await;
    let pa_out = pa_r.output.clone();
    let pa_duration = pa_r.duration_ms;

    if pa_r.status == StageStatus::Failed {
        let err = pa_r
            .error
            .clone()
            .unwrap_or_else(|| "Plan Auditor stage failed".to_string());
        stages.push(pa_r);
        persistence::append_stage_end_event(
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
        persistence::update_run_summary(run_id, run)?;
        return Ok(());
    }
    stages.push(pa_r);
    persistence::append_stage_end_event_with_audit(
        run_id,
        &PipelineStage::PlanAudit,
        iter_num,
        pa_seq + 1,
        &StageEndStatus::Completed,
        pa_duration,
        &pa_out,
    )?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    let parsed = parse_plan_audit_output(&pa_out, &plan_out);
    iter_ctx.audit_verdict = Some(parsed.verdict);
    iter_ctx.audit_reasoning = if parsed.reasoning.trim().is_empty() {
        None
    } else {
        Some(parsed.reasoning)
    };
    iter_ctx.audited_plan = Some(parsed.improved_plan.clone());

    crate::orchestrator::helpers::emit_artifact(app, run_id, "plan_audit", &parsed.improved_plan, iter_num);

    Ok(())
}
