//! Code Fixer stage execution within the review-fix iteration.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::models::{AgentBackend, StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::events;
use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parsing::extract_question;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};
use crate::orchestrator::user_questions::ask_user_question;

/// Runs the Code Fixer stage using the merged review output.
///
/// On success the fixer output is stored in `iter_ctx.fix_output`.
/// Handles failure, pause/cancel checks, and user questions from the fixer.
#[allow(clippy::too_many_arguments)]
pub async fn run_fixer_stage(
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
    enhanced_ref: &str,
    plan_ref: Option<&str>,
    selected_skills_section: Option<&str>,
    judge_feedback: Option<&str>,
    handoff_json: Option<&str>,
    rev_out: &str,
    fixer_agent: &AgentBackend,
    run: &mut PipelineRun,
    stages_vec: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
    workspace_context: &str,
    tracker: &crate::orchestrator::helpers::CliSessionTracker,
) -> Result<(), String> {
    let ws = &request.workspace_path;

    run.current_stage = Some(PipelineStage::CodeFixer);
    let fix_seq = runs::next_sequence(ws, session_id, run_id).unwrap_or(1);
    events::append_stage_start_event(
        ws,
        session_id,
        run_id,
        &PipelineStage::CodeFixer,
        iter_num,
        fix_seq,
    )?;

    // File-reference the merged review to keep prompt size small.
    let review_ref = crate::orchestrator::helpers::file_ref_or_inline(
        ws, session_id, run_id, iter_num, "review", rev_out,
    );

    let fix_input = AgentInput {
        prompt: prompts::build_fixer_user(
            &request.prompt,
            enhanced_ref,
            plan_ref,
            selected_skills_section,
            &review_ref,
            judge_feedback,
            handoff_json,
        ),
        context: Some(super::super::run_setup::compose_agent_context(
            prompts::build_fixer_system(meta),
            workspace_context,
        )),
        workspace_path: request.workspace_path.clone(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(
        ws,
        session_id,
        run_id,
        "code_fixer",
        &fix_input,
        iter_num,
    );

    let fixer_session = tracker.get_ref_for_stage(&PipelineStage::CodeFixer, fixer_agent);
    let fix_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::CodeFixer,
        fixer_agent,
        &fix_input,
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        None,
        fixer_session,
        None,
    )
    .await;
    let fix_out = fix_r.output.clone();
    let fix_duration = fix_r.duration_ms;
    iter_ctx.fix_output = Some(fix_out.clone());

    if fix_r.status == StageStatus::Failed {
        stages_vec.push(fix_r);
        events::handle_stage_failure(
            ws, session_id, run_id,
            &PipelineStage::CodeFixer, iter_num, fix_seq + 1,
            fix_duration, "Code Fixer stage failed", run, stages_vec,
        )?;
        return Ok(());
    }
    stages_vec.push(fix_r);
    events::append_stage_end_event(
        ws, session_id, run_id,
        &PipelineStage::CodeFixer, iter_num, fix_seq + 1,
        &StageEndStatus::Completed, fix_duration,
    )?;

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, iter_num, mem::take(stages_vec));
        return Ok(());
    }

    if let Some(question) = extract_question(&fix_out) {
        iter_ctx.fix_question = Some(question.clone());
        let answer = ask_user_question(
            app,
            ws,
            session_id,
            run_id,
            &PipelineStage::CodeFixer,
            iter_num,
            question,
            fix_out,
            false,
            cancel_flag,
            answer_sender,
        )
        .await?;
        if is_cancelled(cancel_flag) {
            push_cancel_iteration(run, iter_num, mem::take(stages_vec));
            return Ok(());
        }
        if let Some(a) = answer {
            if !a.skipped && !a.answer.is_empty() {
                iter_ctx.fix_answer = Some(a.answer);
            }
        }
    }

    Ok(())
}
