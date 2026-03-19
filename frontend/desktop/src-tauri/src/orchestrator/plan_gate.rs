//! Plan User Gate: pauses the pipeline after planning to let the user
//! approve, revise, or skip the plan before generation begins.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::models::{RunEvent, StageEndStatus};
use crate::models::*;
use crate::storage::{self, runs};

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::{compose_agent_context, IterationContext};
use crate::orchestrator::stages::execute_agent_stage;
use crate::orchestrator::stages::PauseHandling;
use crate::orchestrator::user_questions::{ask_user_question, ask_user_question_with_timeout};

/// Runs the plan gate. Returns `true` if the iteration loop should break
/// (only on cancellation).
#[allow(clippy::too_many_arguments)]
pub async fn run_plan_gate(
    app: &AppHandle,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    run_id: &str,
    iter_num: u32,
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
                app,
                run_id,
                &PipelineStage::PlanAudit,
                iter_num,
                question,
                plan_text.clone(),
                true,
                cancel_flag,
                answer_sender,
                settings.plan_auto_approve_timeout_sec as u64,
            )
            .await?
        } else {
            ask_user_question(
                app,
                run_id,
                &PipelineStage::PlanAudit,
                iter_num,
                question,
                plan_text.clone(),
                true,
                cancel_flag,
                answer_sender,
            )
            .await?
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
            // Log plan approval as event
            let seq = runs::next_sequence(run_id).unwrap_or(1);
            let event = RunEvent::Question {
                v: 1,
                seq,
                ts: storage::now_rfc3339(),
                stage: PipelineStage::PlanAudit,
                iteration: iter_num,
                question: "Plan approval".to_string(),
                answer: format!("approved (after {} revisions)", revisions),
                skipped: false,
            };
            let _ = runs::append_event(run_id, event);
            return Ok(false);
        }

        if action_lower == "skip" {
            iter_ctx.audited_plan = None;
            iter_ctx.planner_plan = None;

            // Log plan skip as event
            let seq = runs::next_sequence(run_id).unwrap_or(1);
            let event = RunEvent::Question {
                v: 1,
                seq,
                ts: storage::now_rfc3339(),
                stage: PipelineStage::PlanAudit,
                iteration: iter_num,
                question: "Plan approval".to_string(),
                answer: format!("skipped (after {} revisions)", revisions),
                skipped: true,
            };
            let _ = runs::append_event(run_id, event);
            return Ok(false);
        }

        // User provided revision feedback.
        revisions += 1;
        if revisions > max_revisions {
            // Log max revisions reached
            let seq = runs::next_sequence(run_id).unwrap_or(1);
            let event = RunEvent::Question {
                v: 1,
                seq,
                ts: storage::now_rfc3339(),
                stage: PipelineStage::PlanAudit,
                iteration: iter_num,
                question: "Plan approval".to_string(),
                answer: format!("approved_max_revisions ({revisions} revisions)"),
                skipped: false,
            };
            let _ = runs::append_event(run_id, event);
            return Ok(false);
        }

        if wait_if_paused(pause_flag, cancel_flag).await {
            push_cancel_iteration(run, iter_num, mem::take(stages));
            return Ok(true);
        }
        // Re-plan with user feedback.
        let user_feedback = action.to_string();
        run.current_stage = Some(PipelineStage::Plan);

        let plan_seq = runs::next_sequence(run_id).unwrap_or(1);
        append_stage_start_event(run_id, &PipelineStage::Plan, iter_num, plan_seq)?;

        let gate_output_path = runs::artifact_output_path(run_id, iter_num, "plan").ok();
        let gate_output_path_str = gate_output_path.as_ref().map(|p| p.to_string_lossy().to_string());

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
                    &iter_ctx.original_prompt,
                    enhanced,
                    iter_ctx.selected_plan(),
                    Some(&user_feedback),
                ),
                context: Some(compose_agent_context(
                    prompts::build_planner_system(meta),
                    workspace_context,
                )),
                workspace_path: run.workspace_path.clone(),
            },
            settings,
            cancel_flag,
            pause_flag,
            PauseHandling::ResumeWithinStage,
            None,
            gate_output_path_str.as_deref(),
        )
        .await;

        let revised_plan = plan_r.output.clone();
        let plan_duration = plan_r.duration_ms;

        if plan_r.status == StageStatus::Failed {
            append_stage_end_event(
                run_id,
                &PipelineStage::Plan,
                iter_num,
                plan_seq + 1,
                &StageEndStatus::Failed,
                plan_duration,
            )?;
            // Log revision failure
            let seq = runs::next_sequence(run_id).unwrap_or(1);
            let event = RunEvent::Question {
                v: 1,
                seq,
                ts: storage::now_rfc3339(),
                stage: PipelineStage::PlanAudit,
                iteration: iter_num,
                question: "Plan approval".to_string(),
                answer: format!("approved_revision_failed ({revisions} revisions)"),
                skipped: false,
            };
            let _ = runs::append_event(run_id, event);
            return Ok(false);
        }

        append_stage_end_event(
            run_id,
            &PipelineStage::Plan,
            iter_num,
            plan_seq + 1,
            &StageEndStatus::Completed,
            plan_duration,
        )?;

        iter_ctx.planner_plan = Some(revised_plan.clone());
        iter_ctx.audited_plan = Some(revised_plan.clone());
    }
}

/// Appends a stage_start event to the event log.
fn append_stage_start_event(
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
) -> Result<(), String> {
    let event = RunEvent::StageStart {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
    };
    runs::append_event(run_id, event)
}

/// Appends a stage_end event to the event log.
fn append_stage_end_event(
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
    status: &StageEndStatus,
    duration_ms: u64,
) -> Result<(), String> {
    let event = RunEvent::StageEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
        status: status.clone(),
        duration_ms,
        verdict: None,
    };
    runs::append_event(run_id, event)
}
