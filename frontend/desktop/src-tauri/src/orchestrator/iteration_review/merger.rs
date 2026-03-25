//! Review Merger stage: merges findings from multiple parallel reviewers.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{AgentBackend, StageEndStatus, *};
use crate::storage::runs;

use crate::orchestrator::helpers::events;
use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::IterationContext;
use crate::orchestrator::stages::{execute_agent_stage, PauseHandling};

/// Runs the Review Merger stage that combines outputs from multiple reviewers.
///
/// `review_texts` contains `(label, output_text, output_kind)` triples from
/// each successful reviewer.
#[allow(clippy::too_many_arguments)]
pub async fn run_review_merger_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    run_id: &str,
    iter_num: u32,
    review_texts: &[(String, String, String)],
    _meta: &PromptMeta,
    workspace_context: &str,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    session_id: &str,
    enhanced: &str,
    run: &mut PipelineRun,
    stages_vec: &mut Vec<StageResult>,
    iter_ctx: &mut IterationContext,
) -> Result<String, String> {
    let merger_backend = settings
        .review_merger_agent
        .as_ref()
        .or(settings.code_reviewer_agent.as_ref())
        .unwrap_or(&AgentBackend::Claude);
    run.current_stage = Some(PipelineStage::ReviewMerge);
    let merger_seq =
        runs::next_sequence(&request.workspace_path, session_id, run_id).unwrap_or(1);
    events::append_stage_start_event(
        &request.workspace_path,
        session_id,
        run_id,
        &PipelineStage::ReviewMerge,
        iter_num,
        merger_seq,
    )?;

    let merger_output_path = runs::artifact_output_path(
        &request.workspace_path,
        session_id,
        run_id,
        iter_num,
        "review",
    )
    .ok();
    let merger_output_path_str = merger_output_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let refs = crate::orchestrator::helpers::build_iteration_refs(
        &request.workspace_path, session_id, run_id, iter_num, enhanced, iter_ctx,
    );

    // File-reference each individual review using the stored output_kind.
    let review_refs: Vec<(String, String)> = review_texts
        .iter()
        .map(|(label, text, output_kind)| {
            let ref_text = crate::orchestrator::helpers::file_ref_or_inline(
                &request.workspace_path,
                session_id,
                run_id,
                iter_num,
                output_kind,
                text,
            );
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
                &refs.enhanced_ref,
                &review_refs,
                refs.plan_ref.as_deref(),
            ),
            context: Some(super::super::run_setup::compose_agent_context(
                prompts::build_review_merger_system(
                    _meta,
                    Some(&runs::artifact_relative_path(
                        session_id, run_id, iter_num, "review",
                    )),
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
        None,
    )
    .await;
    let merged_out = merger_r.output.clone();
    let merger_duration = merger_r.duration_ms;

    if merger_r.status == StageStatus::Failed {
        let err = merger_r
            .error
            .clone()
            .unwrap_or_else(|| "Review Merger failed".into());
        stages_vec.push(merger_r);
        events::handle_stage_failure(
            &request.workspace_path, session_id, run_id,
            &PipelineStage::ReviewMerge, iter_num, merger_seq + 1,
            merger_duration, &err, run, stages_vec,
        )?;
        return Ok(String::new());
    }

    crate::orchestrator::helpers::emit_artifact(
        app, &request.workspace_path, session_id, run_id,
        "review", &merged_out, iter_num,
    );
    events::append_stage_end_event(
        &request.workspace_path, session_id, run_id,
        &PipelineStage::ReviewMerge, iter_num, merger_seq + 1,
        &StageEndStatus::Completed, merger_duration,
    )?;
    stages_vec.push(merger_r);

    Ok(merged_out)
}
