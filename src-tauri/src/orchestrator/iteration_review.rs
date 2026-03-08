//! Review, fix, and judge sub-stages for a single iteration.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::*;
use super::parsing::{extract_question, parse_judge_verdict};
use super::prompts::{self, PromptMeta};
use super::run_setup::*;
use super::stages::*;
use super::user_questions::*;

/// Review + Fix stages (including diffs and user questions).
#[allow(clippy::too_many_arguments)]
pub async fn run_review_fix_stages(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: &DbPool,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    iteration_db_id: i32,
    meta: &PromptMeta,
    enhanced: &str,
    judge_feedback: Option<&str>,
    handoff_json: Option<&str>,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
) -> Result<(), String> {
    run.current_stage = Some(PipelineStage::DiffAfterGenerate);
    stages.push(execute_diff_stage(app, run_id, iter_num, iteration_db_id, PipelineStage::DiffAfterGenerate, &request.workspace_path, db));
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    run.current_stage = Some(PipelineStage::Review);
    let rev_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Review, &settings.reviewer_agent,
        &AgentInput {
            prompt: prompts::build_reviewer_user(&request.prompt, enhanced, iter_ctx.selected_plan()),
            context: Some(prompts::build_reviewer_system(meta)),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let rev_out = rev_r.output.clone();
    iter_ctx.review_output = Some(rev_out.clone());
    iter_ctx.review_user_guidance = None;
    persist_iteration_context(db, run_id, iter_num, iter_ctx);
    emit_artifact(app, run_id, "review", &rev_out, iter_num, db);
    if rev_r.status == StageStatus::Failed {
        stages.push(rev_r);
        run.iterations.push(Iteration { number: iter_num, stages: mem::take(stages), verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Code Reviewer / Auditor stage failed".to_string());
        return Ok(());
    }
    stages.push(rev_r);
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    run.current_stage = Some(PipelineStage::Fix);
    let fix_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Fix, &settings.fixer_agent,
        &AgentInput {
            prompt: prompts::build_fixer_user(&request.prompt, enhanced, iter_ctx.selected_plan(), &rev_out, judge_feedback, handoff_json),
            context: Some(prompts::build_fixer_system(meta)),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let fix_out = fix_r.output.clone();
    iter_ctx.fix_output = Some(fix_out.clone());
    persist_iteration_context(db, run_id, iter_num, iter_ctx);
    if fix_r.status == StageStatus::Failed {
        stages.push(fix_r);
        run.iterations.push(Iteration { number: iter_num, stages: mem::take(stages), verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Code Fixer stage failed".to_string());
        return Ok(());
    }
    stages.push(fix_r);
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    if let Some(question) = extract_question(&fix_out) {
        iter_ctx.fix_question = Some(question.clone());
        persist_iteration_context(db, run_id, iter_num, iter_ctx);
        let answer = ask_user_question(app, run_id, &PipelineStage::Fix, iter_num, question, fix_out.clone(), false, cancel_flag, answer_sender, db).await?;
        if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }
        if let Some(a) = answer {
            if !a.skipped && !a.answer.is_empty() {
                iter_ctx.fix_answer = Some(a.answer);
                persist_iteration_context(db, run_id, iter_num, iter_ctx);
            }
        }
    }

    run.current_stage = Some(PipelineStage::DiffAfterFix);
    stages.push(execute_diff_stage(app, run_id, iter_num, iteration_db_id, PipelineStage::DiffAfterFix, &request.workspace_path, db));
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, mem::take(stages)); return Ok(()); }

    Ok(())
}

/// Judge stage: evaluate completion and parse handoff.
#[allow(clippy::too_many_arguments)]
pub async fn run_judge_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    db: &DbPool,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    iteration_db_id: i32,
    meta: &PromptMeta,
    enhanced: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
    previous_judge_output: &mut Option<String>,
    last_handoff: &mut Option<prompts::IterationHandoff>,
) -> Result<bool, String> {
    let rev_out = iter_ctx.review_output.clone().unwrap_or_default();
    let fix_out = iter_ctx.fix_output.clone().unwrap_or_default();

    run.current_stage = Some(PipelineStage::Judge);
    let judge_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Judge, &settings.final_judge_agent,
        &AgentInput {
            prompt: prompts::build_judge_user(&request.prompt, enhanced, iter_ctx.selected_plan(), &rev_out, &fix_out, previous_judge_output.as_deref()),
            context: Some(prompts::build_judge_system(meta)),
            workspace_path: request.workspace_path.clone(),
        },
        settings, Some(session_id), db,
    ).await;
    let judge_out = judge_r.output.clone();
    iter_ctx.judge_output = Some(judge_out.clone());
    persist_iteration_context(db, run_id, iter_num, iter_ctx);
    emit_artifact(app, run_id, "judge", &judge_out, iter_num, db);
    if judge_r.status == StageStatus::Failed {
        stages.push(judge_r);
        run.iterations.push(Iteration { number: iter_num, stages: mem::take(stages), verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Judge stage failed".to_string());
        return Ok(true);
    }
    stages.push(judge_r);

    let (verdict, reasoning) = parse_judge_verdict(&judge_out);
    let verdict_str = match &verdict { JudgeVerdict::Complete => "COMPLETE", JudgeVerdict::NotComplete => "NOT COMPLETE" };
    let _ = db::runs::update_iteration_verdict(db, run_id, iter_num as i32, Some(verdict_str), Some(&reasoning));
    run.iterations.push(Iteration { number: iter_num, stages: mem::take(stages), verdict: Some(verdict.clone()), judge_reasoning: Some(reasoning) });

    if verdict == JudgeVerdict::Complete {
        run.final_verdict = Some(JudgeVerdict::Complete);
        run.status = PipelineStatus::Completed;
        return Ok(true);
    }

    *previous_judge_output = Some(judge_out.clone());
    let task_brief: String = request.prompt.chars().take(200).collect();
    *last_handoff = Some(prompts::parse_handoff(&judge_out).unwrap_or_else(|| prompts::build_fallback_handoff(&task_brief, &judge_out, iter_num)));

    if iter_num == settings.max_iterations {
        run.final_verdict = Some(JudgeVerdict::NotComplete);
        run.status = PipelineStatus::Completed;
    }

    Ok(false)
}
