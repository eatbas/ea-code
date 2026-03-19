//! Generator (Coder) stage for iteration.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::parsing::extract_question;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};
use crate::orchestrator::helpers::{emit_stage, is_cancelled, push_cancel_iteration};
use crate::orchestrator::user_questions::ask_user_question;

/// Runs the generator/coder stage.
#[allow(clippy::too_many_arguments)]
pub async fn run_generate_stage(
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
) -> Result<String, String> {
    let generator_agent = settings.coder_agent.as_ref().ok_or_else(|| {
        "Coder is not set. Go to Settings/Agents and configure the minimum roles.".to_string()
    })?;

    run.current_stage = Some(PipelineStage::Coder);
    let gen_seq = runs::next_sequence(run_id).unwrap_or(1);
    super::stages::append_stage_start_event(run_id, &PipelineStage::Coder, iter_num, gen_seq)?;
    
    let gen_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::Coder,
        generator_agent,
        &AgentInput {
            prompt: prompts::build_generator_user(
                &request.prompt,
                enhanced,
                iter_ctx.selected_plan(),
                selected_skills_section,
                judge_feedback,
                handoff_json,
            ),
            context: Some(crate::orchestrator::run_setup::compose_agent_context(
                prompts::build_generator_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        None,
    )
    .await;

    let gen_out = gen_r.output.clone();
    let gen_duration = gen_r.duration_ms;
    
    if gen_r.status == StageStatus::Failed {
        stages.push(gen_r);
        super::stages::append_stage_end_event(
            run_id,
            &PipelineStage::Coder,
            iter_num,
            gen_seq + 1,
            &StageEndStatus::Failed,
            gen_duration,
        )?;
        run.iterations
            .push(Iteration { number: iter_num, stages: std::mem::take(stages), verdict: None, judge_reasoning: None });
        run.status = PipelineStatus::Failed;
        run.error = Some("Coder stage failed".to_string());
        super::stages::update_run_summary(run_id, session_id, run)?;
        return Err("Coder stage failed".to_string());
    }
    
    stages.push(gen_r);
    super::stages::append_stage_end_event(
        run_id,
        &PipelineStage::Coder,
        iter_num,
        gen_seq + 1,
        &StageEndStatus::Completed,
        gen_duration,
    )?;
    
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, iter_num, std::mem::take(stages));
        super::stages::update_run_summary(run_id, session_id, run)?;
        return Err("Cancelled".to_string());
    }

    // Handle question extraction
    if let Some(question) = extract_question(&gen_out) {
        iter_ctx.generate_question = Some(question.clone());

        let answer = ask_user_question(
            app, run_id, &PipelineStage::Coder, iter_num, question, gen_out.clone(), false,
            cancel_flag, answer_sender,
        )
        .await?;
        
        if is_cancelled(cancel_flag) {
            push_cancel_iteration(run, iter_num, std::mem::take(stages));
            super::stages::update_run_summary(run_id, session_id, run)?;
            return Err("Cancelled".to_string());
        }
        
        if let Some(ref a) = answer {
            if !a.skipped && !a.answer.is_empty() {
                iter_ctx.generate_answer = Some(a.answer.clone());
                emit_stage(app, run_id, &PipelineStage::Coder, &StageStatus::Completed, iter_num);
            }
        }
    }

    Ok(gen_out)
}
