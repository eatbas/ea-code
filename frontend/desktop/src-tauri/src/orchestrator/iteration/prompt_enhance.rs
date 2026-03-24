//! Prompt enhancement stage for iteration.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::stages::{execute_agent_stage, execute_skipped_stage, PauseHandling};

/// Runs the prompt enhancement stage for iteration 1.
/// Returns the enhanced prompt or error.
pub async fn run_prompt_enhance_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    meta: &PromptMeta,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
) -> Result<String, String> {
    let prompt_enhancer_agent = settings.prompt_enhancer_agent.as_ref().ok_or_else(|| {
        "Prompt Enhancer is not set. Go to Settings/Agents and configure the minimum roles."
            .to_string()
    })?;

    run.current_stage = Some(PipelineStage::PromptEnhance);
    let seq_start = runs::next_sequence(run_id).unwrap_or(1);
    super::stages::append_stage_start_event(
        run_id,
        &PipelineStage::PromptEnhance,
        iter_num,
        seq_start,
    )?;

    let output_path = runs::artifact_output_path(run_id, iter_num, "enhanced_prompt").ok();
    let output_path_str = output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let input = AgentInput {
        prompt: prompts::build_prompt_enhancer_user(&request.prompt),
        // Keep prompt enhancement rewrite-only; avoid workspace execution context here.
        context: Some(prompts::build_prompt_enhancer_system(meta)),
        workspace_path: request.workspace_path.clone(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(run_id, "enhanced_prompt", &input, iter_num);

    let pe_result = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::PromptEnhance,
        prompt_enhancer_agent,
        &input,
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        output_path_str.as_deref(),
        None,
    )
    .await;

    let enhanced = crate::orchestrator::run_setup::normalise_enhanced_prompt(
        &pe_result.output,
        &request.prompt,
    );
    let duration_ms = pe_result.duration_ms;

    if pe_result.status != StageStatus::Failed {
        crate::orchestrator::helpers::emit_artifact(
            app,
            run_id,
            "enhanced_prompt",
            &enhanced,
            iter_num,
        );
        if let Ok(mut summary) = runs::read_summary(run_id) {
            summary.enhanced_prompt = Some(enhanced.clone());
            let _ = runs::update_summary(run_id, &summary);
        }
    }

    if pe_result.status == StageStatus::Failed {
        stages.push(pe_result);
        super::stages::append_stage_end_event(
            run_id,
            &PipelineStage::PromptEnhance,
            iter_num,
            seq_start + 1,
            &StageEndStatus::Failed,
            duration_ms,
        )?;
        run.iterations.push(Iteration {
            number: iter_num,
            stages: std::mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("Prompt Enhancer stage failed".to_string());
        super::stages::update_run_summary(run_id, session_id, run)?;
        return Err("Prompt Enhancer stage failed".to_string());
    }

    stages.push(pe_result);
    super::stages::append_stage_end_event(
        run_id,
        &PipelineStage::PromptEnhance,
        iter_num,
        seq_start + 1,
        &StageEndStatus::Completed,
        duration_ms,
    )?;

    Ok(enhanced)
}

/// Returns a skipped stage result for iterations > 1.
pub fn skip_prompt_enhance(
    app: &AppHandle,
    run_id: &str,
    iter_num: u32,
    original_prompt: &str,
) -> (StageResult, String) {
    let stage = execute_skipped_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::PromptEnhance,
        "Prompt enhancement skipped for iteration > 1; using original prompt.",
    );
    (stage, original_prompt.to_string())
}
