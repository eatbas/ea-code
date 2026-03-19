//! Single iteration logic for the orchestration pipeline.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::models::*;
// runs module used via stages::update_run_summary

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::iteration_planning::run_planning_stages;
use crate::orchestrator::iteration_review::{run_judge_stage, run_review_fix_stages};
use crate::orchestrator::plan_gate::run_plan_gate;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::skill_stage::run_skill_selection_stage;

mod generate;
mod prompt_enhance;
pub mod stages;

/// Mutable state carried across iterations (judge feedback, enhanced prompt, etc.).
pub struct IterationCarryover {
    pub previous_judge_output: Option<String>,
    pub last_handoff: Option<prompts::IterationHandoff>,
    pub persistent_enhanced_prompt: Option<String>,
    pub last_approved_plan: Option<String>,
}

impl IterationCarryover {
    pub fn new() -> Self {
        Self {
            previous_judge_output: None,
            last_handoff: None,
            persistent_enhanced_prompt: None,
            last_approved_plan: None,
        }
    }
}

/// Runs a single iteration of the pipeline. Returns `true` if the loop
/// should break (completion, failure, or cancellation).
#[allow(clippy::too_many_arguments)]
pub async fn run_iteration(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    run: &mut PipelineRun,
    carry: &mut IterationCarryover,
    workspace_context: &str,
) -> Result<bool, String> {
    run.current_iteration = iter_num;
    let mut stages: Vec<StageResult> = Vec::new();
    let mut iter_ctx = IterationContext::new(request.prompt.clone());
    iter_ctx.seed_prior_context(
        carry.persistent_enhanced_prompt.as_deref(),
        carry.last_approved_plan.as_deref(),
    );

    let prior_handoff = carry
        .last_handoff
        .as_ref()
        .map(prompts::render_handoff_for_prompt);

    let meta = PromptMeta {
        iteration: iter_num,
        max_iterations: settings.max_iterations,
        previous_judge_output: prior_handoff
            .as_deref()
            .map(|o| prompts::truncate_judge_output(o, 3000)),
    };

    let judge_feedback = prior_handoff
        .as_deref()
        .map(|o| prompts::truncate_judge_output(o, 3000));
    let handoff_json = carry
        .last_handoff
        .as_ref()
        .and_then(|h| serde_json::to_string_pretty(h).ok());

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, stages);
        return Ok(true);
    }

    // --- 1. Prompt enhance ---
    let enhanced = if iter_num == 1 {
        match prompt_enhance::run_prompt_enhance_stage(
            app,
            request,
            settings,
            cancel_flag,
            pause_flag,
            run_id,
            session_id,
            iter_num,
            &meta,
            run,
            &mut stages,
        )
        .await
        {
            Ok(enhanced) => enhanced,
            Err(_) => return Ok(true),
        }
    } else {
        let seed_prompt = carry
            .persistent_enhanced_prompt
            .as_deref()
            .unwrap_or(&request.prompt);
        let (stage, prompt) =
            prompt_enhance::skip_prompt_enhance(app, run_id, iter_num, seed_prompt);
        stages.push(stage);
        prompt
    };
    iter_ctx.enhanced_prompt = enhanced.clone();
    carry.persistent_enhanced_prompt = Some(enhanced.clone());

    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, stages);
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }

    // --- 2-3. Plan + Plan Audit ---
    run_planning_stages(
        app,
        request,
        settings,
        cancel_flag,
        pause_flag,
        run_id,
        session_id,
        iter_num,
        &meta,
        &enhanced,
        judge_feedback.as_deref(),
        run,
        &mut stages,
        &mut iter_ctx,
        workspace_context,
    )
    .await?;
    carry.last_approved_plan = iter_ctx.selected_plan().map(|plan| plan.to_string());

    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }

    // --- Plan gate: user approval if enabled ---
    if settings.require_plan_approval && iter_ctx.selected_plan().is_some() {
        let should_break = run_plan_gate(
            app,
            settings,
            cancel_flag,
            pause_flag,
            answer_sender,
            run_id,
            iter_num,
            &meta,
            &enhanced,
            workspace_context,
            run,
            &mut stages,
            &mut iter_ctx,
        )
        .await?;
        if should_break {
            stages::update_run_summary(run_id, session_id, run)?;
            return Ok(true);
        }
        carry.last_approved_plan = iter_ctx.selected_plan().map(|plan| plan.to_string());
    }

    // --- 3.5 Skill selection (optional) ---
    let selected_skills_section = run_skill_selection_stage(
        app,
        request,
        settings,
        cancel_flag,
        pause_flag,
        run_id,
        session_id,
        iter_num,
        &meta,
        &enhanced,
        iter_ctx.selected_plan(),
        judge_feedback.as_deref(),
        workspace_context,
        run,
        &mut stages,
    )
    .await?;

    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }
    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, stages);
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, stages);
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }

    // --- 4. Generate ---
    if let Err(_) = generate::run_generate_stage(
        app,
        request,
        settings,
        cancel_flag,
        pause_flag,
        answer_sender,
        run_id,
        session_id,
        iter_num,
        &meta,
        &enhanced,
        selected_skills_section.as_deref(),
        judge_feedback.as_deref(),
        handoff_json.as_deref(),
        run,
        &mut stages,
        &mut iter_ctx,
        workspace_context,
    )
    .await
    {
        return Ok(true);
    }

    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }

    // --- 5-8. Review and Fix stages (diff stages removed) ---
    run_review_fix_stages(
        app,
        request,
        settings,
        cancel_flag,
        pause_flag,
        answer_sender,
        run_id,
        session_id,
        iter_num,
        &meta,
        &enhanced,
        selected_skills_section.as_deref(),
        judge_feedback.as_deref(),
        handoff_json.as_deref(),
        run,
        &mut stages,
        &mut iter_ctx,
        workspace_context,
    )
    .await?;

    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }

    // --- 9. Judge ---
    run_judge_stage(
        app,
        request,
        settings,
        cancel_flag,
        pause_flag,
        run_id,
        session_id,
        iter_num,
        &meta,
        &enhanced,
        run,
        &mut stages,
        &mut iter_ctx,
        &mut carry.previous_judge_output,
        &mut carry.last_handoff,
        workspace_context,
    )
    .await
}
