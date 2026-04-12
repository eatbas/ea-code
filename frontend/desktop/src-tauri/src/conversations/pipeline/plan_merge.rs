use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{PipelineAgent, PipelineStageRecord};
use crate::storage::now_rfc3339;

use super::prompts::{agent_label, build_plan_edit_prompt, build_plan_merge_prompt};
use super::stage_runner::{run_stage, StageConfig};

/// Run the plan-merge stage with user feedback.
pub async fn run_plan_merge_with_feedback(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    feedback: String,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    run_plan_merge_inner(
        app,
        conversation_id,
        workspace_path,
        abort,
        score_id_slot,
        output_buffer,
        stage_index,
        planner_count,
        provider_session_ref,
        agent,
        Some(feedback),
    )
    .await
}

/// Run the plan-merge stage. Resumes the first planner's Symphony session
/// and instructs it to read all individual plans and produce a merged plan.
pub async fn run_plan_merge(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    run_plan_merge_inner(
        app,
        conversation_id,
        workspace_path,
        abort,
        score_id_slot,
        output_buffer,
        stage_index,
        planner_count,
        provider_session_ref,
        agent,
        None,
    )
    .await
}

async fn run_plan_merge_inner(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    feedback: Option<String>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);

    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let plan_dir = format!("{conv_dir}/plan");
    let merged_dir = format!("{conv_dir}/plan_merged");

    if let Err(e) = std::fs::create_dir_all(&merged_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index,
                "Plan Merge".to_string(),
                label,
                Some(now_rfc3339()),
            ),
            format!("Failed to create plan_merged directory: {e}"),
        ));
    }

    // When editing, remove the old merged plan so the file watcher can
    // detect when the agent writes a fresh version.
    if feedback.is_some() {
        let old_file = format!("{merged_dir}/plan_merged.md");
        let _ = std::fs::remove_file(&old_file);
    }

    let prompt = if let Some(ref fb) = feedback {
        build_plan_edit_prompt(fb, &merged_dir)
    } else {
        build_plan_merge_prompt(planner_count, &plan_dir, &merged_dir)
    };

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name: "Plan Merge".to_string(),
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{merged_dir}/plan_merged.md"),
            mode: "resume",
            provider_session_ref: Some(provider_session_ref),
            failure_message: "Plan Merge did not produce a merged plan".to_string(),
            agent_label: label,
            file_required: true,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}
