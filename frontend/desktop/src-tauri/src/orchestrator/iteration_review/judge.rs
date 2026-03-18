//! Judge stage implementation for iteration review.

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::parsing::{parse_judge_verdict, parse_review_findings};
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::execute_agent_stage;

/// Judge stage: evaluate completion and parse handoff.
#[allow(clippy::too_many_arguments)]
pub async fn run_judge_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
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
    let findings = iter_ctx
        .review_findings
        .clone()
        .unwrap_or_else(|| parse_review_findings(&iter_ctx.review_output.clone().unwrap_or_default()));

    run.current_stage = Some(PipelineStage::Judge);
    let judge_seq = runs::next_sequence(run_id).unwrap_or(1);
    super::stages::append_stage_start_event(run_id, &PipelineStage::Judge, iter_num, judge_seq)?;

    let judge_output_path = runs::artifact_output_path(run_id, iter_num, "judge").ok();
    let judge_output_path_str = judge_output_path.as_ref().map(|p| p.to_string_lossy().to_string());

    let judge_r = execute_agent_stage(
        app,
        run_id,
        iter_num,
        PipelineStage::Judge,
        judge_agent,
        &AgentInput {
            // Pass compact findings instead of raw outputs
            prompt: prompts::build_judge_user(
                &request.prompt,
                enhanced,
                iter_ctx.selected_plan(),
                &findings,
                previous_judge_output.as_deref(),
            ),
            context: Some(crate::orchestrator::run_setup::compose_agent_context(
                prompts::build_judge_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        Some(session_id),
        judge_output_path_str.as_deref(),
    )
    .await;
    let judge_out = judge_r.output.clone();
    let judge_duration = judge_r.duration_ms;
    iter_ctx.judge_output = Some(judge_out.clone());

    if judge_r.status != StageStatus::Failed {
        crate::orchestrator::helpers::emit_artifact(app, run_id, "judge", &judge_out, iter_num);
    }

    if judge_r.status == StageStatus::Failed {
        stages.push(judge_r);
        super::stages::append_stage_end_event(
            run_id,
            &PipelineStage::Judge,
            iter_num,
            judge_seq + 1,
            &StageEndStatus::Failed,
            judge_duration,
        )?;
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("Judge stage failed".to_string());
        super::stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }
    stages.push(judge_r);

    let (verdict, reasoning) = parse_judge_verdict(&judge_out);

    // Append stage_end with verdict
    super::stages::append_stage_end_event_with_verdict(
        run_id,
        &PipelineStage::Judge,
        iter_num,
        judge_seq + 1,
        &StageEndStatus::Completed,
        judge_duration,
        Some(verdict.clone()),
    )?;

    // Append iteration_end event
    let iter_seq = runs::next_sequence(run_id).unwrap_or(1);
    let iter_event = crate::models::RunEvent::IterationEnd {
        v: 1,
        seq: iter_seq,
        ts: crate::storage::now_rfc3339(),
        iteration: iter_num,
        verdict: verdict.clone(),
    };
    let _ = runs::append_event(run_id, iter_event);

    run.iterations.push(Iteration {
        number: iter_num,
        stages: mem::take(stages),
        verdict: Some(verdict.clone()),
        judge_reasoning: Some(reasoning.clone()),
    });

    if verdict == JudgeVerdict::Complete {
        run.final_verdict = Some(JudgeVerdict::Complete);
        run.status = PipelineStatus::Completed;
        super::stages::update_run_summary(run_id, session_id, run)?;
        return Ok(true);
    }

    *previous_judge_output = Some(judge_out.clone());
    let task_brief: String = request.prompt.chars().take(200).collect();
    *last_handoff = Some(
        prompts::parse_handoff(&judge_out)
            .unwrap_or_else(|| prompts::build_fallback_handoff(&task_brief, &judge_out, iter_num)),
    );

    if iter_num == settings.max_iterations {
        run.final_verdict = Some(JudgeVerdict::NotComplete);
        run.status = PipelineStatus::Completed;
    }

    super::stages::update_run_summary(run_id, session_id, run)?;
    Ok(false)
}
