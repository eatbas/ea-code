//! Plan User Gate: pauses the pipeline after planning to let the user
//! approve, revise, or skip the plan before generation begins.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::*;
use super::prompts::{self, PromptMeta};
use super::run_setup::*;
use super::stages::*;
use super::user_questions::*;

/// Runs the plan gate. Returns `true` if the iteration loop should break
/// (only on cancellation).
#[allow(clippy::too_many_arguments)]
pub async fn run_plan_gate(
    app: &AppHandle,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: &DbPool,
    run_id: &str,
    iter_num: u32,
    iteration_db_id: i32,
    meta: &PromptMeta,
    enhanced: &str,
    workspace_context: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
) -> Result<bool, String> {
    let mut revisions = 0u32;
    let max_revisions = settings.max_plan_revisions;

    loop {
        if wait_if_paused(pause_flag, cancel_flag).await {
            push_cancel_iteration(run, iter_num, mem::take(stages));
            return Ok(true);
        }
        let plan_text = iter_ctx.selected_plan().unwrap_or_default().to_string();
        let question = format!(
            "The plan is ready for your review.\n\n\
             Reply with:\n\
             - \"approve\" to proceed with this plan\n\
             - \"skip\" to discard the plan and generate without one\n\
             - Any other text to provide revision feedback ({rev} of {max} revisions used)\n\n\
             Plan:\n{plan}",
            rev = revisions,
            max = max_revisions,
            plan = plan_text,
        );

        let answer = if settings.plan_auto_approve_timeout_sec > 0 {
            ask_user_question_with_timeout(
                app, run_id, &PipelineStage::PlanAudit, iter_num,
                question, plan_text.clone(), true,
                cancel_flag, answer_sender, db,
                settings.plan_auto_approve_timeout_sec as u64,
            ).await?
        } else {
            ask_user_question(
                app, run_id, &PipelineStage::PlanAudit, iter_num,
                question, plan_text.clone(), true,
                cancel_flag, answer_sender, db,
            ).await?
        };

        if is_cancelled(cancel_flag) {
            push_cancel_iteration(run, iter_num, mem::take(stages));
            return Ok(true);
        }

        let action = match &answer {
            Some(a) if a.skipped => "approve",
            Some(a) => a.answer.trim(),
            None => "approve", // Timeout or channel drop → auto-approve
        };

        let action_lower = action.to_lowercase();

        if action_lower == "approve" || action_lower.is_empty() {
            let _ = db::runs::update_iteration_plan_approval(
                db, run_id, iter_num as i32, "approved", revisions as i32,
            );
            return Ok(false);
        }

        if action_lower == "skip" {
            iter_ctx.audited_plan = None;
            iter_ctx.planner_plan = None;

            let _ = db::runs::update_iteration_plan_approval(
                db, run_id, iter_num as i32, "skipped", revisions as i32,
            );
            return Ok(false);
        }

        // User provided revision feedback.
        revisions += 1;
        if revisions > max_revisions {
            let _ = db::runs::update_iteration_plan_approval(
                db, run_id, iter_num as i32, "approved_max_revisions", revisions as i32,
            );
            return Ok(false);
        }

        if wait_if_paused(pause_flag, cancel_flag).await {
            push_cancel_iteration(run, iter_num, mem::take(stages));
            return Ok(true);
        }
        // Re-plan with user feedback.
        let user_feedback = action.to_string();
        run.current_stage = Some(PipelineStage::Plan);
        let plan_r = execute_agent_stage(
            app, run_id, iter_num, iteration_db_id, PipelineStage::Plan,
            settings.planner_agent.as_ref().unwrap_or(&crate::models::AgentBackend::Claude),
            &AgentInput {
                prompt: prompts::build_planner_user(
                    &iter_ctx.original_prompt, enhanced,
                    iter_ctx.selected_plan(),
                    Some(&user_feedback),
                ),
                context: Some(compose_agent_context(
                    prompts::build_planner_system(meta),
                    workspace_context,
                )),
                workspace_path: run.workspace_path.clone(),
            },
            settings, cancel_flag, None, db,
        ).await;

        let revised_plan = plan_r.output.clone();
        if plan_r.status == StageStatus::Failed {
            let _ = db::runs::update_iteration_plan_approval(
                db, run_id, iter_num as i32, "approved_revision_failed", revisions as i32,
            );
            return Ok(false);
        }

        iter_ctx.planner_plan = Some(revised_plan.clone());
        iter_ctx.audited_plan = Some(revised_plan.clone());
        emit_artifact(app, run_id, "plan_revised", &revised_plan, iter_num, db);
    }
}
