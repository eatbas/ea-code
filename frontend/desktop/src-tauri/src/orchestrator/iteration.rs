//! Single iteration logic for the orchestration pipeline.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::*;
use super::iteration_planning::run_planning_stages;
use super::iteration_review::{run_judge_stage, run_review_fix_stages};
use super::parsing::extract_question;
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
    pause_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: &DbPool,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    run: &mut PipelineRun,
    previous_judge_output: &mut Option<String>,
    last_handoff: &mut Option<prompts::IterationHandoff>,
    workspace_context: &str,
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
    let generator_agent = settings.coder_agent.as_ref().ok_or_else(|| {
        "Coder is not set. Go to Settings/Agents and configure the minimum roles.".to_string()
    })?;

    if wait_if_paused(pause_flag, cancel_flag).await { push_cancel_iteration(run, iter_num, stages); return Ok(true); }

    // --- 1. Prompt enhance ---
    let enhanced = if iter_num == 1 {
        let prompt_enhancer_agent = settings
            .prompt_enhancer_agent
            .as_ref()
            .ok_or_else(|| {
                "Prompt Enhancer is not set. Go to Settings/Agents and configure the minimum roles."
                    .to_string()
            })?;
        run.current_stage = Some(PipelineStage::PromptEnhance);
        let pe_result = execute_agent_stage(
            app, run_id, iter_num, iteration_db_id, PipelineStage::PromptEnhance,
            prompt_enhancer_agent,
            &AgentInput {
                prompt: prompts::build_prompt_enhancer_user(&request.prompt),
                // Keep prompt enhancement rewrite-only; avoid workspace execution context here.
                context: Some(prompts::build_prompt_enhancer_system(&meta)),
                workspace_path: request.workspace_path.clone(),
            },
            settings, Some(session_id), db,
        ).await;
        let enhanced = normalise_enhanced_prompt(&pe_result.output, &request.prompt);
        if pe_result.status == StageStatus::Failed {
            stages.push(pe_result);
            run.iterations.push(Iteration { number: iter_num, stages, verdict: None, judge_reasoning: None });
            run.status = PipelineStatus::Failed;
            run.error = Some("Prompt Enhancer stage failed".to_string());
            return Ok(true);
        }
        stages.push(pe_result);
        enhanced
    } else {
        stages.push(execute_skipped_stage(
            app,
            run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::PromptEnhance,
            "Prompt enhancement skipped for iteration > 1; using original prompt.",
            db,
        ));
        request.prompt.clone()
    };
    iter_ctx.enhanced_prompt = enhanced.clone();

    emit_artifact(app, run_id, "enhanced_prompt", &enhanced, iter_num, db);
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }

    // --- 2-3. Plan + Plan Audit ---
    run_planning_stages(
        app, request, settings, cancel_flag, pause_flag, db,
        run_id, session_id, iter_num, iteration_db_id,
        &meta, &enhanced, judge_feedback.as_deref(),
        run, &mut stages, &mut iter_ctx, workspace_context,
    ).await?;
    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        return Ok(true);
    }

    // --- Plan gate: user approval if enabled ---
    if settings.require_plan_approval && iter_ctx.selected_plan().is_some() {
        let should_break = run_plan_gate(
            app, settings, cancel_flag, pause_flag, answer_sender, db,
            run_id, iter_num, iteration_db_id,
            &meta, &enhanced, workspace_context,
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
        workspace_context,
        run,
        &mut stages,
    )
    .await?;
    if run.status == PipelineStatus::Failed || run.status == PipelineStatus::Cancelled {
        return Ok(true);
    }
    if wait_if_paused(pause_flag, cancel_flag).await { push_cancel_iteration(run, iter_num, stages); return Ok(true); }
    if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }

    // --- 4. Generate ---
    run.current_stage = Some(PipelineStage::Coder);
    let gen_r = execute_agent_stage(
        app, run_id, iter_num, iteration_db_id, PipelineStage::Coder,
        generator_agent,
        &AgentInput {
            prompt: prompts::build_generator_user(
                &request.prompt,
                &enhanced,
                iter_ctx.selected_plan(),
                selected_skills_section.as_deref(),
                judge_feedback.as_deref(),
                handoff_json.as_deref(),
            ),
            context: Some(compose_agent_context(
                prompts::build_generator_system(&meta),
                workspace_context,
            )),
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
    
        let answer = ask_user_question(app, run_id, &PipelineStage::Coder, iter_num, question, gen_out.clone(), false, cancel_flag, answer_sender, db).await?;
        if is_cancelled(cancel_flag) { push_cancel_iteration(run, iter_num, stages); return Ok(true); }
        if let Some(ref a) = answer {
            if !a.skipped && !a.answer.is_empty() {
                iter_ctx.generate_answer = Some(a.answer.clone());
            
                emit_stage(app, run_id, &PipelineStage::Coder, &StageStatus::Completed, iter_num, db);
            }
        }
    }

    // --- 5-8. Diff, Review, Fix, Diff ---
    run_review_fix_stages(
        app, request, settings, cancel_flag, pause_flag, answer_sender, db,
        run_id, session_id, iter_num, iteration_db_id,
        &meta, &enhanced, selected_skills_section.as_deref(), judge_feedback.as_deref(), handoff_json.as_deref(),
        run, &mut stages, &mut iter_ctx, workspace_context,
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
        previous_judge_output, last_handoff, workspace_context,
    ).await
}
