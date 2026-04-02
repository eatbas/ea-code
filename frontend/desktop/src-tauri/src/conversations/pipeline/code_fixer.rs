use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{PipelineAgent, PipelineStageRecord};
use crate::storage::now_rfc3339;

use super::prompts::{agent_label, build_code_fixer_prompt};
use super::stage_runner::{run_stage, StageConfig};

/// Run the Code Fixer stage. Resumes the coder's session and applies
/// fixes based on the consolidated review.
pub async fn run_code_fixer(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    coder_session_ref: String,
    agent: PipelineAgent,
    code_fixer_dir_override: Option<String>,
    review_merged_path_override: Option<String>,
    stage_name_override: Option<String>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let code_fixer_dir =
        code_fixer_dir_override.unwrap_or_else(|| format!("{conv_dir}/code_fixer"));
    let review_merged_path = review_merged_path_override
        .unwrap_or_else(|| format!("{conv_dir}/review_merged/review_merged.md"));
    let stage_name = stage_name_override.unwrap_or_else(|| "Code Fixer".to_string());

    if let Err(e) = std::fs::create_dir_all(&code_fixer_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index,
                stage_name.clone(),
                label,
                Some(now_rfc3339()),
            ),
            format!("Failed to create code_fixer directory: {e}"),
        ));
    }

    let prompt = build_code_fixer_prompt(&review_merged_path, &code_fixer_dir);

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
            file_to_watch: format!("{code_fixer_dir}/code_fixer_done.md"),
            mode: "resume",
            provider_session_ref: Some(coder_session_ref),
            failure_message: "Code Fixer did not produce a completion summary".to_string(),
            agent_label: label,
            file_required: true,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}
