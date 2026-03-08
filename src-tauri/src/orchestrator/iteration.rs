//! Single iteration logic for the orchestration pipeline.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::*;
use super::iteration_review::{run_judge_stage, run_review_fix_stages};
use super::parsing::{extract_question, parse_plan_audit_output};
use super::plan_gate::run_plan_gate;
use super::prompts::{self, PromptMeta};
use super::run_setup::*;
use super::skill_stage::run_skill_selection_stage;
use super::stages::*;
use super::user_questions::*;

/// Runs a single iteration of the pipeline. Returns `true` if the loop
/// should break (completion, failure, or cancellation).
#[allow(clippy::too_many_arguments)]
pub async fn run_iteration(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: &DbPool,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    run: &mut PipelineRun,
    previous_judge_output: &mut Option<String>,
    last_handoff: &mut Option<prompts::IterationHandoff>,
) -> Result<bool, String> {
    run.current_iteration = iter_num;
    let mut stages: Vec<StageResult> = Vec::new();
    let iteration_db_id = db::runs::insert_iteration(db, run_id, iter_num as i32)?;
    let mut iter_ctx = IterationContext::new(request.prompt.clone());

    let meta = PromptMeta {
        iteration: iter_num,
        max_iterations: settings.max_iterations,
        previous_judge_output: previous_judge_output
            .as_deref()
            .map(|o| prompts::truncate_judge_output(o, 3000)),
    };

    let judge_feedback = previous_judge_output
        .as_deref()
        .map(|o| prompts::truncate_judge_output(o, 3000));
    let handoff_json = last_handoff
        .as_ref()
        .and_then(|h| serde_json::to_string_pretty(h).ok());

    // --- 1. Prompt enhance ---
    run.current_stage = Some(PipelineStage::PromptEnhance);
    let pe_result = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::PromptEnhance,
        &settings.prompt_enhancer_agent,
        &AgentInput {
            prompt: prompts::build_prompt_enhancer_user(&request.prompt),
            context: Some(prompts::build_prompt_enhancer_system(&meta)),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let enhanced = normalise_enhanced_prompt(&pe_result.output, &request.prompt);
    iter_ctx.enhanced_prompt = enhanced.clone();
    persist_iteration_context(db, run_id, iter_num, &iter_ctx);
    if pe_result.status == StageStatus::Failed {
        stages.push(pe_result);
        run.iterations.push(Iteration { number: iter_num, stages, verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Prompt Enhancer stage failed".to_string());
        return Ok(true);
    }
    stages.push(pe_result);
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }

    // --- 2-3. Plan + Plan Audit ---
    run_planning_stages(
        app, request, settings, cancel_flag, db,
        run_id, session_id, iter_num, iteration_db_id,
        &meta, &enhanced, judge_feedback.as_deref(),
        run, &mut stages, &mut iter_ctx,
    ).await?;
    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        return Ok(true);
    }

    // --- Plan gate: user approval if enabled ---
    if settings.require_plan_approval && iter_ctx.selected_plan().is_some() {
        let should_break = run_plan_gate(
            app, settings, cancel_flag, answer_sender, db,
            run_id, iter_num, iteration_db_id,
            &meta, &enhanced,
            run, &mut stages, &mut iter_ctx,
        ).await?;
        if should_break {
            return Ok(true);
        }
    }

    // --- 3.5 Skill selection (optional) ---
    let selected_skills_section = run_skill_selection_stage(
        app,
        request,
        settings,
        db,
        run_id,
        session_id,
        iter_num,
        iteration_db_id,
        &meta,
        &enhanced,
        iter_ctx.selected_plan(),
        judge_feedback.as_deref(),
        run,
        &mut stages,
    )
    .await?;
    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        return Ok(true);
    }
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }

    // --- 4. Generate ---
    run.current_stage = Some(PipelineStage::Generate);
    let gen_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Generate,
        &settings.generator_agent,
        &AgentInput {
            prompt: prompts::build_generator_user(
                &request.prompt,
                &enhanced,
                iter_ctx.selected_plan(),
                selected_skills_section.as_deref(),
                judge_feedback.as_deref(),
                handoff_json.as_deref(),
            ),
            context: Some(prompts::build_generator_system(&meta)),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let gen_out = gen_r.output.clone();
    if gen_r.status == StageStatus::Failed {
        stages.push(gen_r);
        run.iterations.push(Iteration { number: iter_num, stages, verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Coder stage failed".to_string());
        return Ok(true);
    }
    stages.push(gen_r);
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }

    if let Some(question) = extract_question(&gen_out) {
        iter_ctx.generate_question = Some(question.clone());
        persist_iteration_context(db, run_id, iter_num, &iter_ctx);
        let answer = ask_user_question(app, run_id, &PipelineStage::Generate, iter_num, question, gen_out.clone(), false, cancel_flag, answer_sender, db).await?;
        if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }
        if let Some(ref a) = answer {
            if !a.skipped && !a.answer.is_empty() {
                iter_ctx.generate_answer = Some(a.answer.clone());
                persist_iteration_context(db, run_id, iter_num, &iter_ctx);
                emit_stage(app, run_id, &PipelineStage::Generate, &StageStatus::Completed, iter_num);
            }
        }
    }

    // --- 5-8. Diff, Review, Fix, Diff ---
    run_review_fix_stages(
        app, request, settings, cancel_flag, answer_sender, db,
        run_id, session_id, iter_num, iteration_db_id,
        &meta, &enhanced, selected_skills_section.as_deref(), judge_feedback.as_deref(), handoff_json.as_deref(),
        run, &mut stages, &mut iter_ctx,
    ).await?;
    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        return Ok(true);
    }

    // --- 9. Judge ---
    run_judge_stage(
        app, request, settings, db,
        run_id, session_id, iter_num, iteration_db_id,
        &meta, &enhanced,
        run, &mut stages, &mut iter_ctx,
        previous_judge_output, last_handoff,
    ).await
}

/// Planning stages: Plan + Plan Audit.
#[allow(clippy::too_many_arguments)]
async fn run_planning_stages(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
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
) -> Result<(), String> {
    let planning_enabled = settings.planner_agent.is_some() && settings.plan_auditor_agent.is_some();
    if !planning_enabled {
        let skip = "Planner and Plan Auditor must both be selected; skipping planning stages.";
        stages.push(execute_skipped_stage(app, run_id, iter_num, iteration_db_id, PipelineStage::Plan, skip, db));
        stages.push(execute_skipped_stage(app, run_id, iter_num, iteration_db_id, PipelineStage::PlanAudit, skip, db));
        return Ok(());
    }

    let plan_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Plan,
        settings.planner_agent.as_ref().unwrap_or(&crate::models::AgentBackend::Claude),
        &AgentInput {
            prompt: prompts::build_planner_user(&request.prompt, enhanced, iter_ctx.selected_plan(), judge_feedback),
            context: Some(prompts::build_planner_system(meta)),
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
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    let pa_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::PlanAudit,
        settings.plan_auditor_agent.as_ref().unwrap_or(&crate::models::AgentBackend::Claude),
        &AgentInput {
            prompt: prompts::build_plan_auditor_user(&request.prompt, enhanced, &plan_out, None, None),
            context: Some(prompts::build_plan_auditor_system(meta)),
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
