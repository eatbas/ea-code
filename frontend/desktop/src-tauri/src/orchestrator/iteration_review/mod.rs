//! Review, fix, and judge sub-stages for a single iteration.
//! Supports 1-N parallel reviewers with optional Review Merger.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::models::{AgentBackend, StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::{is_cancelled, push_cancel_iteration, wait_if_paused};
use crate::orchestrator::parallel_stage::{
    run_parallel_stage_tasks, ParallelStageRun, ParallelStageSlot, ParallelStageTask,
};
use crate::orchestrator::parsing::{extract_question, parse_review_findings};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};
use crate::orchestrator::user_questions::ask_user_question;

mod judge;
pub mod stages;

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

fn stage_failed_due_to_pause(result: &StageResult) -> bool {
    if result.status != StageStatus::Failed {
        return false;
    }
    result
        .error
        .as_ref()
        .map(|err| err.to_ascii_lowercase().contains("paused by user"))
        .unwrap_or(false)
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

    // --- Run reviewers (1 sequential, 2+ parallel) ---
    // File-reference the enhanced prompt and plan to keep prompt size small.
    let ws = &request.workspace_path;
    let enhanced_ref = crate::orchestrator::helpers::artifact_file_path(
        ws, session_id, run_id, iter_num, "enhanced_prompt",
    )
    .map(|p| crate::orchestrator::helpers::file_ref(&p))
    .unwrap_or_else(|| enhanced.to_string());

    let plan_ref = iter_ctx.selected_plan().and_then(|_| {
        let kind = if iter_ctx.audited_plan.is_some() {
            "plan_audit"
        } else {
            "plan"
        };
        crate::orchestrator::helpers::artifact_file_path(ws, session_id, run_id, iter_num, kind)
            .map(|p| crate::orchestrator::helpers::file_ref(&p))
    });

    let reviewer_user_prompt = prompts::build_reviewer_user(
        &request.prompt,
        &enhanced_ref,
        plan_ref.as_deref(),
        judge_feedback,
    );
    let reviewer_output_rel = runs::artifact_relative_path(session_id, run_id, iter_num, "review");
    let reviewer_context = super::run_setup::compose_agent_context(
        prompts::build_reviewer_system(meta, Some(&reviewer_output_rel)),
        workspace_context,
    );

    let rev_out = if reviewer_slots.len() == 1 {
        run_single_reviewer(
            app,
            run_id,
            iter_num,
            &reviewer_slots[0],
            &reviewer_user_prompt,
            &reviewer_context,
            &request.workspace_path,
            settings,
            cancel_flag,
            pause_flag,
            session_id,
            run,
            stages,
        )
        .await?
    } else {
        run_parallel_reviewers_and_merge(
            app,
            request,
            run_id,
            iter_num,
            &reviewer_slots,
            &reviewer_user_prompt,
            &reviewer_context,
            settings,
            cancel_flag,
            pause_flag,
            session_id,
            meta,
            enhanced,
            workspace_context,
            run,
            stages,
            iter_ctx,
        )
        .await?
    };

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
    let findings = parse_review_findings(&rev_out);
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
    run.current_stage = Some(PipelineStage::CodeFixer);
    let fix_seq = runs::next_sequence(ws, session_id, run_id).unwrap_or(1);
    stages::append_stage_start_event(ws, session_id, run_id, &PipelineStage::CodeFixer, iter_num, fix_seq)?;

    // File-reference the merged review to keep prompt size small.
    let review_ref = crate::orchestrator::helpers::artifact_file_path(
        ws, session_id, run_id, iter_num, "review",
    )
    .map(|p| crate::orchestrator::helpers::file_ref(&p))
    .unwrap_or_else(|| rev_out.clone());

    let fix_input = AgentInput {
        prompt: prompts::build_fixer_user(
            &request.prompt,
            &enhanced_ref,
            plan_ref.as_deref(),
            selected_skills_section,
            &review_ref,
            judge_feedback,
            handoff_json,
        ),
        context: Some(super::run_setup::compose_agent_context(
            prompts::build_fixer_system(meta),
            workspace_context,
        )),
        workspace_path: request.workspace_path.clone(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(ws, session_id, run_id, "code_fixer", &fix_input, iter_num);

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
        None,
    )
    .await;
    let fix_out = fix_r.output.clone();
    let fix_duration = fix_r.duration_ms;
    iter_ctx.fix_output = Some(fix_out.clone());

    if fix_r.status == StageStatus::Failed {
        stages.push(fix_r);
        stages::append_stage_end_event(
            ws,
            session_id,
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
        stages::update_run_summary(ws, session_id, run_id, run)?;
        return Ok(());
    }
    stages.push(fix_r);
    stages::append_stage_end_event(
        ws,
        session_id,
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
            push_cancel_iteration(run, iter_num, mem::take(stages));
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

/// Runs a single reviewer and returns its output text. Empty string on failure.
#[allow(clippy::too_many_arguments)]
async fn run_single_reviewer(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    slot: &ParallelStageSlot,
    user_prompt: &str,
    context: &str,
    workspace_path: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
) -> Result<String, String> {
    run.current_stage = Some(PipelineStage::CodeReviewer);
    let rev_seq = runs::next_sequence(workspace_path, session_id, run_id).unwrap_or(1);
    stages::append_stage_start_event(workspace_path, session_id, run_id, &PipelineStage::CodeReviewer, iter_num, rev_seq)?;

    let rev_output_path = runs::artifact_output_path(workspace_path, session_id, run_id, iter_num, "review").ok();
    let rev_output_path_str = rev_output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let rev_input = AgentInput {
        prompt: user_prompt.to_string(),
        context: Some(context.to_string()),
        workspace_path: workspace_path.to_string(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(workspace_path, session_id, run_id, "review", &rev_input, iter_num);

    let rev_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        slot.stage.clone(),
        &slot.backend,
        &rev_input,
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        rev_output_path_str.as_deref(),
        None,
    )
    .await;
    let rev_out = rev_r.output.clone();
    let rev_duration = rev_r.duration_ms;

    if rev_r.status != StageStatus::Failed {
        crate::orchestrator::helpers::emit_artifact(app, workspace_path, session_id, run_id, "review", &rev_out, iter_num);
    }

    if rev_r.status == StageStatus::Failed {
        stages.push(rev_r);
        stages::append_stage_end_event(
            workspace_path,
            session_id,
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
        run.error = Some("Code Reviewer stage failed".to_string());
        stages::update_run_summary(workspace_path, session_id, run_id, run)?;
        return Ok(String::new());
    }
    stages.push(rev_r);
    stages::append_stage_end_event(
        workspace_path,
        session_id,
        run_id,
        &PipelineStage::CodeReviewer,
        iter_num,
        rev_seq + 1,
        &StageEndStatus::Completed,
        rev_duration,
    )?;

    Ok(rev_out)
}

/// Runs 2+ reviewers in parallel, then runs the Review Merger stage.
#[allow(clippy::too_many_arguments)]
async fn run_parallel_reviewers_and_merge(
    app: &AppHandle,
    request: &PipelineRequest,
    run_id: &str,
    iter_num: u32,
    slots: &[ParallelStageSlot],
    user_prompt: &str,
    context: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    meta: &PromptMeta,
    enhanced: &str,
    workspace_context: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
) -> Result<String, String> {
    let review_texts = loop {
        let tasks: Vec<ParallelStageTask> = slots
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let app = app.clone();
                let backend = slot.backend.clone();
                let task_stage = slot.stage.clone();
                let future_stage = task_stage.clone();
                let prompt = user_prompt.to_string();
                let ctx = context.to_string();
                let ws = request.workspace_path.clone();
                let settings = settings.clone();
                let cf = cancel_flag.clone();
                let pf = pause_flag.clone();
                let sid = session_id.to_string();
                let rid = run_id.to_string();

                let output_kind = format!("review_{}", i + 1);
                let output_path = runs::artifact_output_path(&ws, &sid, &rid, iter_num, &output_kind).ok();
                let output_path_str = output_path.map(|p| p.to_string_lossy().to_string());

                ParallelStageTask {
                    stage: task_stage,
                    output_kind,
                    future: Box::pin(async move {
                        execute_agent_stage(
                            &app,
                            &rid,
                            iter_num,
                            future_stage,
                            &backend,
                            &AgentInput {
                                prompt,
                                context: Some(ctx),
                                workspace_path: ws,
                            },
                            &settings,
                            &cf,
                            &pf,
                            PauseHandling::ReturnPausedError,
                            Some(&sid),
                            output_path_str.as_deref(),
                            None,
                        )
                        .await
                    }),
                }
            })
            .collect();

        let stage_checkpoint = stages.len();
        let mut pause_retry_requested = false;
        let ws_ref = request.workspace_path.clone();
        let sid_ref = session_id.to_string();
        let successful = run_parallel_stage_tasks(
            run_id,
            iter_num,
            tasks,
            |rid, stage, iteration, seq| stages::append_stage_start_event(&ws_ref, &sid_ref, rid, stage, iteration, seq),
            |rid, stage, iteration, seq, status, dur| stages::append_stage_end_event(&ws_ref, &sid_ref, rid, stage, iteration, seq, status, dur),
            |parallel_run| {
                let ParallelStageRun {
                    index,
                    output_kind,
                    result,
                    ..
                } = parallel_run;
                let paused = stage_failed_due_to_pause(&result);
                let failed = result.status == StageStatus::Failed;
                let output = result.output.clone();
                if paused {
                    pause_retry_requested = true;
                }
                stages.push(result);

                if failed {
                    None
                } else {
                    Some((
                        format!("Review from Reviewer {}", index + 1),
                        output_kind,
                        output,
                    ))
                }
            },
            &request.workspace_path,
            session_id,
        )
        .await?;

        if pause_retry_requested {
            stages.truncate(stage_checkpoint);
            for idx in 0..slots.len() {
                let kind = format!("review_{}", idx + 1);
                if let Ok(path) = runs::artifact_output_path(&request.workspace_path, session_id, run_id, iter_num, &kind) {
                    let _ = std::fs::remove_file(path);
                }
            }
            if wait_if_paused(pause_flag, cancel_flag).await {
                return Ok(String::new());
            }
            continue;
        }

        let mut merged_review_texts = Vec::new();
        for (title, output_kind, output) in successful {
            crate::orchestrator::helpers::emit_artifact(
                app,
                &request.workspace_path,
                session_id,
                run_id,
                &output_kind,
                &output,
                iter_num,
            );
            merged_review_texts.push((title, output));
        }
        break merged_review_texts;
    };

    if review_texts.is_empty() {
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("All reviewer stages failed".to_string());
        stages::update_run_summary(&request.workspace_path, session_id, run_id, run)?;
        return Ok(String::new());
    }

    // --- Review Merger stage ---
    let merger_backend = settings
        .review_merger_agent
        .as_ref()
        .or(settings.code_reviewer_agent.as_ref())
        .unwrap_or(&AgentBackend::Claude);
    run.current_stage = Some(PipelineStage::ReviewMerge);
    let merger_seq = runs::next_sequence(&request.workspace_path, session_id, run_id).unwrap_or(1);
    stages::append_stage_start_event(&request.workspace_path, session_id, run_id, &PipelineStage::ReviewMerge, iter_num, merger_seq)?;

    let merger_output_path = runs::artifact_output_path(&request.workspace_path, session_id, run_id, iter_num, "review").ok();
    let merger_output_path_str = merger_output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    // File-reference the enhanced prompt and plan to keep prompt size small.
    let enhanced_ref = crate::orchestrator::helpers::artifact_file_path(
        &request.workspace_path, session_id, run_id, iter_num, "enhanced_prompt",
    )
    .map(|p| crate::orchestrator::helpers::file_ref(&p))
    .unwrap_or_else(|| enhanced.to_string());

    let plan_ref = iter_ctx.selected_plan().and_then(|_| {
        let kind = if iter_ctx.audited_plan.is_some() {
            "plan_audit"
        } else {
            "plan"
        };
        crate::orchestrator::helpers::artifact_file_path(&request.workspace_path, session_id, run_id, iter_num, kind)
            .map(|p| crate::orchestrator::helpers::file_ref(&p))
    });

    // File-reference each individual review.
    let review_refs: Vec<(String, String)> = review_texts
        .iter()
        .enumerate()
        .map(|(i, (label, text))| {
            let kind = format!("review_{}", i + 1);
            let ref_text =
                crate::orchestrator::helpers::artifact_file_path(&request.workspace_path, session_id, run_id, iter_num, &kind)
                    .map(|p| crate::orchestrator::helpers::file_ref(&p))
                    .unwrap_or_else(|| text.clone());
            (label.clone(), ref_text)
        })
        .collect();

    let merger_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::ReviewMerge,
        merger_backend,
        &AgentInput {
            prompt: prompts::build_review_merger_user(
                &request.prompt,
                &enhanced_ref,
                &review_refs,
                plan_ref.as_deref(),
            ),
            context: Some(super::run_setup::compose_agent_context(
                prompts::build_review_merger_system(
                    meta,
                    Some(&runs::artifact_relative_path(session_id, run_id, iter_num, "review")),
                ),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        merger_output_path_str.as_deref(),
        None,
    )
    .await;
    let merged_out = merger_r.output.clone();

    if merger_r.status == StageStatus::Failed {
        let err = merger_r
            .error
            .clone()
            .unwrap_or_else(|| "Review Merger failed".into());
        stages::append_stage_end_event(
            &request.workspace_path,
            session_id,
            run_id,
            &PipelineStage::ReviewMerge,
            iter_num,
            merger_seq + 1,
            &StageEndStatus::Failed,
            merger_r.duration_ms,
        )?;
        stages.push(merger_r);
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some(err);
        stages::update_run_summary(&request.workspace_path, session_id, run_id, run)?;
        return Ok(String::new());
    }

    crate::orchestrator::helpers::emit_artifact(app, &request.workspace_path, session_id, run_id, "review", &merged_out, iter_num);
    stages::append_stage_end_event(
        &request.workspace_path,
        session_id,
        run_id,
        &PipelineStage::ReviewMerge,
        iter_num,
        merger_seq + 1,
        &StageEndStatus::Completed,
        merger_r.duration_ms,
    )?;
    stages.push(merger_r);

    Ok(merged_out)
}
