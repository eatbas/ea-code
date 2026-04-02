use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{PipelineAgent, PipelineStageRecord};
use crate::storage::now_rfc3339;

use super::prompts::{agent_label, build_coder_prompt};
use super::stage_runner::{run_stage, StageConfig};

/// Run the single Coder stage. Starts a new session and implements the
/// approved merged plan.
pub async fn run_coder(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let coder_dir = format!("{conv_dir}/coder");
    let merged_plan = format!("{conv_dir}/plan_merged/plan_merged.md");

    if let Err(e) = std::fs::create_dir_all(&coder_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index,
                "Coder".to_string(),
                label,
                Some(now_rfc3339()),
            ),
            format!("Failed to create coder directory: {e}"),
        ));
    }

    let prompt = build_coder_prompt(&merged_plan, &coder_dir);

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name: "Coder".to_string(),
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{coder_dir}/coder_done.md"),
            mode: "new",
            provider_session_ref: None,
            failure_message: "Coder did not produce a completion summary".to_string(),
            agent_label: label,
            file_required: true,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}
