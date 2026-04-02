use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{PipelineAgent, PipelineStageRecord};
use crate::storage::now_rfc3339;

use super::prompts::{agent_label, build_review_merge_prompt};
use super::stage_runner::{run_stage, StageConfig};

/// Run the review-merge stage. Resumes the first reviewer's session
/// and instructs it to read all individual reviews and produce a
/// consolidated review.
pub async fn run_review_merge(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    reviewer_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    review_dir_override: Option<String>,
    review_merged_dir_override: Option<String>,
    stage_name_override: Option<String>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let review_dir = review_dir_override.unwrap_or_else(|| format!("{conv_dir}/review"));
    let review_merged_dir = review_merged_dir_override.unwrap_or_else(|| format!("{conv_dir}/review_merged"));
    let stage_name = stage_name_override.unwrap_or_else(|| "Review Merge".to_string());

    if let Err(e) = std::fs::create_dir_all(&review_merged_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index, stage_name.clone(), label, Some(now_rfc3339()),
            ),
            format!("Failed to create review_merged directory: {e}"),
        ));
    }

    let prompt = build_review_merge_prompt(reviewer_count, &review_dir, &review_merged_dir);

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name,
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{review_merged_dir}/review_merged.md"),
            mode: "resume",
            provider_session_ref: Some(provider_session_ref),
            failure_message: "Review Merge did not produce a merged review".to_string(),
            agent_label: label,
            file_required: true,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}
