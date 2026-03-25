//! Review, fix, and judge sub-stages for a single iteration.
//! Supports 1-N parallel reviewers with optional Review Merger.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::models::*;

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parallel_stage::ParallelStageSlot;
use crate::orchestrator::prompts::PromptMeta;
use crate::orchestrator::run_setup::IterationContext;

mod fixer;
mod judge;
mod merger;
mod reviewers;

pub use judge::run_judge_stage;

/// Collects active reviewer slots from settings.
fn active_reviewer_slots(settings: &AppSettings) -> Vec<ParallelStageSlot> {
    let mut slots = Vec::new();
    if let Some(b) = &settings.code_reviewer_agent {
        slots.push(ParallelStageSlot {
            backend: b.clone(),
            stage: PipelineStage::CodeReviewer,
        });
    }
    for (i, slot) in settings.extra_reviewers.iter().enumerate() {
        if let Some(b) = &slot.agent {
            slots.push(ParallelStageSlot {
                backend: b.clone(),
                stage: PipelineStage::ExtraReviewer(i as u8),
            });
        }
    }
    slots
}

/// Review + Fix stages (supports 1-N parallel reviewers + optional merger).
#[allow(clippy::too_many_arguments)]
pub async fn run_review_fix_stages(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    meta: &PromptMeta,
    enhanced: &str,
    selected_skills_section: Option<&str>,
    judge_feedback: Option<&str>,
    handoff_json: Option<&str>,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
    workspace_context: &str,
    tracker: &crate::orchestrator::helpers::CliSessionTracker,
) -> Result<(), String> {
    let reviewer_slots = active_reviewer_slots(settings);
    if reviewer_slots.is_empty() {
        return Err(
            "Code Reviewer is not set. Go to Settings/Agents and configure the minimum roles."
                .to_string(),
        );
    }
    let fixer_agent = settings.code_fixer_agent.as_ref().ok_or_else(|| {
        "Code Fixer is not set. Go to Settings/Agents and configure the minimum roles.".to_string()
    })?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Run reviewers (1+ parallel, merger for 2+) ---
    let refs = crate::orchestrator::helpers::build_iteration_refs(
        &request.workspace_path, session_id, run_id, iter_num, enhanced, iter_ctx,
    );

    let reviewer_user_prompt = crate::orchestrator::prompts::build_reviewer_user(
        &request.prompt,
        &refs.enhanced_ref,
        refs.plan_ref.as_deref(),
        judge_feedback,
    );

    let rev_out = reviewers::run_parallel_reviewers_and_merge(
        app,
        request,
        run_id,
        iter_num,
        &reviewer_slots,
        &reviewer_user_prompt,
        meta,
        workspace_context,
        settings,
        cancel_flag,
        pause_flag,
        session_id,
        enhanced,
        run,
        stages,
        iter_ctx,
        tracker,
    )
    .await?;

    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // Empty means all reviewers failed or merger failed.
    if rev_out.is_empty() {
        return Ok(());
    }

    iter_ctx.review_output = Some(rev_out.clone());
    iter_ctx.review_user_guidance = None;
    let findings = crate::orchestrator::parsing::parse_review_findings(&rev_out);
    iter_ctx.review_findings = Some(findings);

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Code Fixer stage ---
    fixer::run_fixer_stage(
        app,
        request,
        settings,
        cancel_flag,
        pause_flag,
        answer_sender,
        run_id,
        session_id,
        iter_num,
        meta,
        &refs.enhanced_ref,
        refs.plan_ref.as_deref(),
        selected_skills_section,
        judge_feedback,
        handoff_json,
        &rev_out,
        fixer_agent,
        run,
        stages,
        iter_ctx,
        workspace_context,
        tracker,
    )
    .await
}
