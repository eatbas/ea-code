use tauri::{AppHandle, Emitter};

use crate::commands::api_health::symphony_base_url;
use crate::http::symphony_client;
use crate::models::{
    ConversationStatus, PipelineStageOutputDelta, PipelineStageStatusEvent,
};

use super::super::super::events::{EVENT_PIPELINE_STAGE_OUTPUT_DELTA, EVENT_PIPELINE_STAGE_STATUS};

pub(super) fn request_symphony_stop(score_id: String) {
    let client = symphony_client().clone();
    let url = format!("{}/v1/chat/{score_id}/stop", symphony_base_url());

    tokio::spawn(async move {
        match client.post(&url).send().await {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                eprintln!("[pipeline] Failed to stop symphony job {score_id}: HTTP {status}: {body}");
            }
            Err(error) => eprintln!("[pipeline] Failed to stop symphony job {score_id}: {error}"),
        }
    });
}

pub(super) fn emit_stage_status(
    app: &AppHandle,
    conversation_id: &str,
    stage_index: usize,
    stage_name: &str,
    status: ConversationStatus,
    agent_label: &str,
    text: Option<String>,
) -> Result<(), String> {
    app.emit(
        EVENT_PIPELINE_STAGE_STATUS,
        PipelineStageStatusEvent {
            conversation_id: conversation_id.to_string(),
            stage_index,
            stage_name: stage_name.to_string(),
            status,
            agent_label: agent_label.to_string(),
            text,
            started_at: None,
            finished_at: None,
        },
    )
    .map_err(|error| format!("Failed to emit pipeline stage status: {error}"))
}

pub(super) fn emit_stage_delta(
    app: &AppHandle,
    conversation_id: &str,
    stage_index: usize,
    text: &str,
) -> Result<(), String> {
    app.emit(
        EVENT_PIPELINE_STAGE_OUTPUT_DELTA,
        PipelineStageOutputDelta {
            conversation_id: conversation_id.to_string(),
            stage_index,
            text: text.to_string(),
        },
    )
    .map_err(|error| format!("Failed to emit pipeline stage delta: {error}"))
}
