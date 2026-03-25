//! Judge stage implementation for iteration review.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::events;
use crate::orchestrator::parsing::{parse_judge_verdict, parse_review_findings};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};

/// Judge stage: evaluate completion and parse handoff.
#[allow(clippy::too_many_arguments)]
pub async fn run_judge_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    meta: &PromptMeta,
    enhanced: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
    previous_judge_output: &mut Option<String>,
    last_handoff: &mut Option<prompts::IterationHandoff>,
    workspace_context: &str,
) -> Result<bool, String> {
    let judge_agent = settings.final_judge_agent.as_ref().ok_or_else(|| {
        "Judge is not set. Go to Settings/Agents and configure the minimum roles.".to_string()
    })?;

    // Use parsed findings instead of raw review output
    let findings = iter_ctx.review_findings.clone().unwrap_or_else(|| {
        parse_review_findings(&iter_ctx.review_output.clone().unwrap_or_default())
    });

    run.current_stage = Some(PipelineStage::Judge);
    let ws = &request.workspace_path;
    let judge_seq = runs::next_sequence(ws, session_id, run_id).unwrap_or(1);
    events::append_stage_start_event(ws, session_id, run_id, &PipelineStage::Judge, iter_num, judge_seq)?;

    let judge_output_path = runs::artifact_output_path(ws, session_id, run_id, iter_num, "judge").ok();
    let judge_output_path_str = judge_output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let refs = crate::orchestrator::helpers::build_iteration_refs(
        ws, session_id, run_id, iter_num, enhanced, iter_ctx,
    );

    let input = AgentInput {
        prompt: prompts::build_judge_user(
            &request.prompt,
            &refs.enhanced_ref,
            refs.plan_ref.as_deref(),
            &findings,
            previous_judge_output.as_deref(),
        ),
        context: Some(crate::orchestrator::run_setup::compose_agent_context(
            prompts::build_judge_system(
                meta,
                Some(&runs::artifact_relative_path(session_id, run_id, iter_num, "judge")),
            ),
            workspace_context,
        )),
        workspace_path: request.workspace_path.clone(),
    };

    crate::orchestrator::helpers::emit_prompt_artifact(ws, session_id, run_id, "judge", &input, iter_num);

    let judge_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::Judge,
        judge_agent,
        &input,
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        judge_output_path_str.as_deref(),
        None,
        None,
    )
    .await;
    let judge_out = judge_r.output.clone();
    let judge_duration = judge_r.duration_ms;
    iter_ctx.judge_output = Some(judge_out.clone());

    if judge_r.status != StageStatus::Failed {
        crate::orchestrator::helpers::emit_artifact(app, ws, session_id, run_id, "judge", &judge_out, iter_num);
    }

    if judge_r.status == StageStatus::Failed {
        stages.push(judge_r);
        events::handle_stage_failure(
            ws, session_id, run_id,
            &PipelineStage::Judge, iter_num, judge_seq + 1,
            judge_duration, "Judge stage failed", run, stages,
        )?;
        return Ok(true);
    }
    stages.push(judge_r);

    let (verdict, reasoning) = parse_judge_verdict(&judge_out);

    // Append stage_end with verdict
    events::append_stage_end_event_with_verdict(
        ws,
        session_id,
        run_id,
        &PipelineStage::Judge,
        iter_num,
        judge_seq + 1,
        &StageEndStatus::Completed,
        judge_duration,
        Some(verdict.clone()),
    )?;

    // Append iteration_end event
    let iter_seq = runs::next_sequence(ws, session_id, run_id).unwrap_or(1);
    let iter_event = crate::models::RunEvent::IterationEnd {
        v: 1,
        seq: iter_seq,
        ts: crate::storage::now_rfc3339(),
        iteration: iter_num,
        verdict: verdict.clone(),
    };
    let _ = runs::append_event(ws, session_id, run_id, iter_event);

    run.iterations.push(Iteration {
        number: iter_num,
        stages: mem::take(stages),
        verdict: Some(verdict.clone()),
        judge_reasoning: Some(reasoning.clone()),
    });

    if verdict == JudgeVerdict::Complete {
        run.final_verdict = Some(JudgeVerdict::Complete);
        run.status = PipelineStatus::Completed;
        events::update_run_summary(ws, session_id, run_id, run)?;
        return Ok(true);
    }

    let task_brief: String = request.prompt.chars().take(200).collect();
    let handoff = prompts::parse_handoff(&judge_out)
        .unwrap_or_else(|| prompts::build_fallback_handoff(&task_brief, &judge_out, iter_num));
    *previous_judge_output = Some(prompts::render_handoff_for_prompt(&handoff));
    *last_handoff = Some(handoff);

    if iter_num == settings.max_iterations {
        run.final_verdict = Some(JudgeVerdict::NotComplete);
        run.status = PipelineStatus::Completed;
    }

    events::update_run_summary(ws, session_id, run_id, run)?;
    Ok(false)
}
