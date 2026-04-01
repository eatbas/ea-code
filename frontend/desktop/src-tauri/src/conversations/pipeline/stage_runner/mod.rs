mod emission;
mod finalise;
mod watchers;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::commands::api_health::symphony_base_url;
use crate::conversations::symphony_request::SymphonyChatRequest;
use crate::conversations::sse::{consume_symphony_sse, SymphonySseEvent};
use crate::models::{ConversationStatus, PipelineStageRecord};
use crate::storage::now_rfc3339;

use self::emission::{emit_stage_delta, request_symphony_stop};
use self::finalise::{
    determine_final_status,
    read_accumulated_output,
    resolve_stage_text,
};
use self::watchers::spawn_stage_watchers;

/// Configuration for a single pipeline stage execution.
pub struct StageConfig {
    pub stage_index: usize,
    pub stage_name: String,
    pub provider: String,
    pub model: String,
    pub prompt: String,
    pub file_to_watch: String,
    pub mode: &'static str,
    pub provider_session_ref: Option<String>,
    pub failure_message: String,
    pub agent_label: String,
    /// When `false`, the stage completes based on SSE stream end alone -
    /// the `file_to_watch` is read if it exists but is not required.
    /// Suitable for coding agents that modify the codebase but may not
    /// write a dedicated marker file.
    pub file_required: bool,
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
    emission::emit_stage_status(
        app,
        conversation_id,
        stage_index,
        stage_name,
        status,
        agent_label,
        text,
    )
}

/// Execute a single pipeline stage: send a request to symphony, stream SSE
/// output, watch for the expected plan file, and persist the result.
pub async fn run_stage(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    config: StageConfig,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let StageConfig {
        stage_index,
        stage_name,
        provider,
        model,
        prompt,
        file_to_watch,
        mode,
        provider_session_ref,
        failure_message,
        agent_label,
        file_required,
    } = config;
    let started_at = now_rfc3339();

    if let Err(error) = emit_stage_status(
        &app,
        &conversation_id,
        stage_index,
        &stage_name,
        ConversationStatus::Running,
        &agent_label,
        None,
    ) {
        return Err((
            PipelineStageRecord::failed(stage_index, stage_name, agent_label, Some(started_at)),
            error,
        ));
    }

    let request = SymphonyChatRequest {
        provider: &provider,
        model: &model,
        workspace_path: &workspace_path,
        mode,
        prompt: &prompt,
        provider_session_ref: provider_session_ref.as_deref(),
        stream: true,
        provider_options: HashMap::new(),
    };

    let emit_failed = |error_message: &str| -> (PipelineStageRecord, String) {
        let _ = emit_stage_status(
            &app,
            &conversation_id,
            stage_index,
            &stage_name,
            ConversationStatus::Failed,
            &agent_label,
            None,
        );
        (
            PipelineStageRecord::failed(
                stage_index,
                stage_name.clone(),
                agent_label.clone(),
                Some(started_at.clone()),
            ),
            error_message.to_string(),
        )
    };

    let url = format!("{}/v1/chat", symphony_base_url());
    let response = match crate::http::symphony_client().post(&url).json(&request).send().await {
        Ok(response) => response,
        Err(error) => {
            return Err(emit_failed(&format!(
                "{stage_name} failed to contact symphony: {error}"
            )));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(emit_failed(&format!(
            "{stage_name}: symphony HTTP {status}: {body}"
        )));
    }

    let watchers = spawn_stage_watchers(file_to_watch.clone(), abort.clone());
    let app_ref = app.clone();
    let conversation_id_ref = conversation_id.clone();
    let score_id_writer = score_id_slot.clone();
    let output_buffer_writer = output_buffer.clone();
    let abort_for_run = abort.clone();
    let local_stop_for_run = watchers.local_stop.clone();
    let session_ref: Arc<std::sync::Mutex<Option<String>>> =
        Arc::new(std::sync::Mutex::new(None));
    let session_ref_writer = session_ref.clone();

    let result = consume_symphony_sse(response, &watchers.local_stop, |event| match event {
        SymphonySseEvent::OutputDelta { text } => {
            if let Ok(mut buffer) = output_buffer_writer.lock() {
                if !buffer.is_empty() {
                    buffer.push('\n');
                }
                buffer.push_str(text);
            }
            emit_stage_delta(&app_ref, &conversation_id_ref, stage_index, text)
        }
        SymphonySseEvent::RunStarted { score_id } => {
            if let Ok(mut guard) = score_id_writer.lock() {
                *guard = Some(score_id.clone());
            }
            if abort_for_run.load(std::sync::atomic::Ordering::Acquire) {
                local_stop_for_run.store(true, std::sync::atomic::Ordering::Release);
                request_symphony_stop(score_id.clone());
            }
            Ok(())
        }
        SymphonySseEvent::ProviderSession { provider_session_ref } => {
            if let Ok(mut guard) = session_ref_writer.lock() {
                *guard = Some(provider_session_ref.clone());
            }
            Ok(())
        }
        _ => Ok(()),
    })
    .await;

    let final_status = determine_final_status(
        abort.as_ref(),
        &file_to_watch,
        watchers.file_ready.as_ref(),
        file_required,
        &stage_name,
        &result,
    );
    let file_text = resolve_stage_text(
        &file_to_watch,
        &output_buffer,
        file_required,
        &final_status,
        &stage_name,
    );
    let captured_session_ref = session_ref.lock().ok().and_then(|guard| guard.clone());
    let captured_job = score_id_slot.lock().ok().and_then(|guard| guard.clone());
    let accumulated_text = read_accumulated_output(&output_buffer);
    let display_text = file_text.or_else(|| {
        if accumulated_text.is_empty() {
            None
        } else {
            Some(accumulated_text.clone())
        }
    });

    let _ = emit_stage_status(
        &app,
        &conversation_id,
        stage_index,
        &stage_name,
        final_status.clone(),
        &agent_label,
        display_text.clone(),
    );

    let record = PipelineStageRecord {
        stage_index,
        stage_name: stage_name.clone(),
        agent_label: agent_label.clone(),
        status: final_status.clone(),
        text: display_text.unwrap_or(accumulated_text),
        started_at: Some(started_at),
        finished_at: Some(now_rfc3339()),
        score_id: captured_job,
        provider_session_ref: captured_session_ref,
    };

    if let Err(error) = crate::conversations::persistence::update_pipeline_stage(
        &workspace_path,
        &conversation_id,
        &record,
    ) {
        eprintln!("[pipeline] Failed to save stage state for {stage_name}: {error}");
    }

    if final_status == ConversationStatus::Failed {
        Err((record, failure_message))
    } else {
        Ok(record)
    }
}
