//! Review, fix, and judge sub-stages for a single iteration.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::parsing::{extract_question, parse_review_findings};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::execute_agent_stage;
use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::user_questions::ask_user_question;

mod judge;
pub mod stages;

pub use judge::run_judge_stage;


/// Review + Fix stages (diff stages removed - agents read git directly).
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
) -> Result<(), String> {
    let reviewer_agent = settings.code_reviewer_agent.as_ref().ok_or_else(|| {
        "Code Reviewer is not set. Go to Settings/Agents and configure the minimum roles."
            .to_string()
    })?;
    let fixer_agent = settings.code_fixer_agent.as_ref().ok_or_else(|| {
        "Code Fixer is not set. Go to Settings/Agents and configure the minimum roles."
            .to_string()
    })?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Code Reviewer stage (diff stage removed) ---
    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    run.current_stage = Some(PipelineStage::CodeReviewer);
    let rev_seq = runs::next_sequence(run_id).unwrap_or(1);
    stages::append_stage_start_event(run_id, &PipelineStage::CodeReviewer, iter_num, rev_seq)?;

    let rev_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::CodeReviewer,
        reviewer_agent,
        &AgentInput {
            prompt: prompts::build_reviewer_user(
                &request.prompt,
                enhanced,
                iter_ctx.selected_plan(),
                judge_feedback,
            ),
            context: Some(super::run_setup::compose_agent_context(
                prompts::build_reviewer_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        Some(session_id),
    )
    .await;
    let rev_out = rev_r.output.clone();
    let rev_duration = rev_r.duration_ms;
    iter_ctx.review_output = Some(rev_out.clone());
    iter_ctx.review_user_guidance = None;

    // Parse findings for later use by Judge
    let findings = parse_review_findings(&rev_out);
    iter_ctx.review_findings = Some(findings.clone());

    if rev_r.status != StageStatus::Failed {
        crate::orchestrator::helpers::emit_artifact(app, run_id, "review", &rev_out, iter_num);
    }

    if rev_r.status == StageStatus::Failed {
        stages.push(rev_r);
        stages::append_stage_end_event(
            run_id,
            &PipelineStage::CodeReviewer,
            iter_num,
            rev_seq + 1,
            &StageEndStatus::Failed,
            rev_duration,
        )?;
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("Code Reviewer / Auditor stage failed".to_string());
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(());
    }
    stages.push(rev_r);
    crate::orchestrator::iteration_review::stages::append_stage_end_event(
        run_id,
        &PipelineStage::CodeReviewer,
        iter_num,
        rev_seq + 1,
        &StageEndStatus::Completed,
        rev_duration,
    )?;
    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    // --- Code Fixer stage ---
    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    run.current_stage = Some(PipelineStage::CodeFixer);
    let fix_seq = runs::next_sequence(run_id).unwrap_or(1);
    stages::append_stage_start_event(run_id, &PipelineStage::CodeFixer, iter_num, fix_seq)?;

    let fix_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::CodeFixer,
        fixer_agent,
        &AgentInput {
            prompt: prompts::build_fixer_user(
                &request.prompt,
                enhanced,
                iter_ctx.selected_plan(),
                selected_skills_section,
                &rev_out,
                judge_feedback,
                handoff_json,
            ),
            context: Some(super::run_setup::compose_agent_context(
                prompts::build_fixer_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        Some(session_id),
    )
    .await;
    let fix_out = fix_r.output.clone();
    let fix_duration = fix_r.duration_ms;
    iter_ctx.fix_output = Some(fix_out.clone());

    if fix_r.status == StageStatus::Failed {
        stages.push(fix_r);
        crate::orchestrator::iteration_review::stages::append_stage_end_event(
            run_id,
            &PipelineStage::CodeFixer,
            iter_num,
            fix_seq + 1,
            &StageEndStatus::Failed,
            fix_duration,
        )?;
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("Code Fixer stage failed".to_string());
        stages::update_run_summary(run_id, session_id, run)?;
        return Ok(());
    }
    stages.push(fix_r);
    crate::orchestrator::iteration_review::stages::append_stage_end_event(
        run_id,
        &PipelineStage::CodeFixer,
        iter_num,
        fix_seq + 1,
        &StageEndStatus::Completed,
        fix_duration,
    )?;
    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    if let Some(question) = extract_question(&fix_out) {
        iter_ctx.fix_question = Some(question.clone());

        let answer = ask_user_question(
            app, run_id, &PipelineStage::CodeFixer, iter_num, question, fix_out.clone(), false,
            cancel_flag, answer_sender,
        )
        .await?;
        if is_cancelled(cancel_flag) {
            push_cancel_iteration(run, iter_num, mem::take(stages));
            return Ok(());
        }
        if let Some(a) = answer {
            if !a.skipped && !a.answer.is_empty() {
                iter_ctx.fix_answer = Some(a.answer);
            }
        }
    }

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, mem::take(stages));
        return Ok(());
    }

    Ok(())
}
