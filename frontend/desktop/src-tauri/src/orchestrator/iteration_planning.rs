//! Planning stages (Plan + Plan Audit) extracted from iteration.rs.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::db::DbPool;
use crate::models::*;

use super::helpers::*;
use super::parsing::parse_plan_audit_output;
use super::prompts::{self, PromptMeta};
use super::run_setup::*;
use super::stages::*;

/// Planning stages: Plan + Plan Audit.
#[allow(clippy::too_many_arguments)]
pub async fn run_planning_stages(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    db: &DbPool,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    iteration_db_id: i32,
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
        stages.push(execute_skipped_stage(app, run_id, iter_num, iteration_db_id, PipelineStage::Plan, skip, db));
        stages.push(execute_skipped_stage(app, run_id, iter_num, iteration_db_id, PipelineStage::PlanAudit, skip, db));
        return Ok(());
    }

    if wait_if_paused(pause_flag, cancel_flag).await { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }
    let plan_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Plan,
        settings.planner_agent.as_ref().unwrap_or(&crate::models::AgentBackend::Claude),
        &AgentInput {
            prompt: prompts::build_planner_user(&request.prompt, enhanced, iter_ctx.selected_plan(), judge_feedback),
            context: Some(compose_agent_context(
                prompts::build_planner_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let plan_out = plan_r.output.clone();
    iter_ctx.planner_plan = Some(plan_out.clone());
    persist_iteration_context(db, run_id, iter_num, iter_ctx);
    emit_artifact(app, run_id, "plan", &plan_out, iter_num, db);
    if plan_r.status == StageStatus::Failed {
        stages.push(plan_r);
        run.iterations.push(Iteration { number: iter_num, stages: mem::take(stages), verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Planner stage failed".to_string());
        return Ok(());
    }
    stages.push(plan_r);
    if wait_if_paused(pause_flag, cancel_flag).await { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    if wait_if_paused(pause_flag, cancel_flag).await { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }
    let pa_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::PlanAudit,
        settings.plan_auditor_agent.as_ref().unwrap_or(&crate::models::AgentBackend::Claude),
        &AgentInput {
            prompt: prompts::build_plan_auditor_user(&request.prompt, enhanced, &plan_out, None, None),
            context: Some(compose_agent_context(
                prompts::build_plan_auditor_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let pa_out = pa_r.output.clone();
    emit_artifact(app, run_id, "plan_audit", &pa_out, iter_num, db);
    if pa_r.status == StageStatus::Failed {
        let err = pa_r.error.clone().unwrap_or_else(|| "Plan Auditor stage failed".to_string());
        stages.push(pa_r);
        run.iterations.push(Iteration { number: iter_num, stages: mem::take(stages), verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some(err);
        return Ok(());
    }
    stages.push(pa_r);
    if wait_if_paused(pause_flag, cancel_flag).await { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    let parsed = parse_plan_audit_output(&pa_out, &plan_out);
    iter_ctx.audit_verdict = Some(parsed.verdict);
    iter_ctx.audit_reasoning = if parsed.reasoning.trim().is_empty() { None } else { Some(parsed.reasoning) };
    iter_ctx.audited_plan = Some(parsed.improved_plan);
    persist_iteration_context(db, run_id, iter_num, iter_ctx);
    if let Some(plan) = iter_ctx.audited_plan.as_ref() {
        emit_artifact(app, run_id, "plan_final", plan, iter_num, db);
    }

    Ok(())
}
