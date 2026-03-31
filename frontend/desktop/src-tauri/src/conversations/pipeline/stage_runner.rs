use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

use crate::commands::api_health::symphony_base_url;
use crate::http::symphony_client;
use crate::models::{
    ConversationStatus, PipelineStageOutputDelta, PipelineStageRecord, PipelineStageStatusEvent,
};
use crate::storage::now_rfc3339;

use super::super::events::{EVENT_PIPELINE_STAGE_OUTPUT_DELTA, EVENT_PIPELINE_STAGE_STATUS};
use super::super::symphony_request::SymphonyChatRequest;
use super::super::sse::{consume_symphony_sse, SymphonySseEvent};

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
    /// When `false`, the stage completes based on SSE stream end alone —
    /// the `file_to_watch` is read if it exists but is not required.
    /// Suitable for coding agents that modify the codebase but may not
    /// write a dedicated marker file.
    pub file_required: bool,
}

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
        },
    )
    .map_err(|e| format!("Failed to emit pipeline stage status: {e}"))
}

fn emit_stage_delta(
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
    .map_err(|e| format!("Failed to emit pipeline stage delta: {e}"))
}

/// Execute a single pipeline stage: send a request to symphony, stream SSE
/// output, watch for the expected plan file, and persist the result.
///
/// This is the unified implementation shared by both planner stages and the
/// plan-merge stage — the caller configures the differences via [`StageConfig`].
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

    // Emit Running status.
    if let Err(e) = emit_stage_status(
        &app, &conversation_id, stage_index, &stage_name,
        ConversationStatus::Running, &agent_label, None,
    ) {
        return Err((
            PipelineStageRecord::failed(stage_index, stage_name, agent_label, Some(started_at)),
            e,
        ));
    }

    // Build and send symphony request.
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

    let emit_failed = |err_msg: &str| -> (PipelineStageRecord, String) {
        let _ = emit_stage_status(
            &app, &conversation_id, stage_index, &stage_name,
            ConversationStatus::Failed, &agent_label, None,
        );
        (
            PipelineStageRecord::failed(
                stage_index, stage_name.clone(), agent_label.clone(), Some(started_at.clone()),
            ),
            err_msg.to_string(),
        )
    };

    let url = format!("{}/v1/chat", symphony_base_url());
    let response = match symphony_client().post(&url).json(&request).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(emit_failed(&format!(
                "{stage_name} failed to contact symphony: {e}"
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

    // Spawn file watcher — polls for the expected output file.
    let file_ready = Arc::new(AtomicBool::new(false));
    let local_stop = Arc::new(AtomicBool::new(false));
    {
        let file_ready_w = file_ready.clone();
        let local_stop_w = local_stop.clone();
        let abort_w = abort.clone();
        let file_path = file_to_watch.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(2)).await;
                if abort_w.load(Ordering::Acquire) {
                    return;
                }
                if Path::new(&file_path).exists() {
                    sleep(Duration::from_secs(3)).await;
                    file_ready_w.store(true, Ordering::Release);
                    local_stop_w.store(true, Ordering::Release);
                    return;
                }
            }
        });
    }

    // Propagate global abort to local stop signal.
    {
        let local_stop_m = local_stop.clone();
        let abort_m = abort.clone();
        tokio::spawn(async move {
            while !local_stop_m.load(Ordering::Acquire) {
                if abort_m.load(Ordering::Acquire) {
                    local_stop_m.store(true, Ordering::Release);
                    return;
                }
                sleep(Duration::from_millis(200)).await;
            }
        });
    }

    // Consume SSE stream.
    let app_ref = app.clone();
    let conv_id = conversation_id.clone();
    let score_id_writer = score_id_slot.clone();
    let buf_writer = output_buffer.clone();
    let abort_for_run = abort.clone();
    let local_stop_for_run = local_stop.clone();
    let session_ref: Arc<std::sync::Mutex<Option<String>>> =
        Arc::new(std::sync::Mutex::new(None));
    let session_ref_writer = session_ref.clone();

    let result = consume_symphony_sse(response, &local_stop, |event| match event {
        SymphonySseEvent::OutputDelta { text } => {
            if let Ok(mut buf) = buf_writer.lock() {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(text);
            }
            emit_stage_delta(&app_ref, &conv_id, stage_index, text)
        }
        SymphonySseEvent::RunStarted { score_id } => {
            if let Ok(mut guard) = score_id_writer.lock() {
                *guard = Some(score_id.clone());
            }
            if abort_for_run.load(Ordering::Acquire) {
                local_stop_for_run.store(true, Ordering::Release);
                request_symphony_stop(score_id.clone());
            }
            Ok(())
        }
        SymphonySseEvent::ProviderSession {
            provider_session_ref,
        } => {
            if let Ok(mut guard) = session_ref_writer.lock() {
                *guard = Some(provider_session_ref.clone());
            }
            Ok(())
        }
        _ => Ok(()),
    })
    .await;

    // Determine final status.
    let file_exists =
        file_ready.load(Ordering::Acquire) || Path::new(&file_to_watch).exists();

    let final_status = if abort.load(Ordering::Acquire) {
        ConversationStatus::Stopped
    } else if file_exists {
        ConversationStatus::Completed
    } else if !file_required {
        // For stages where the file is optional (e.g. Coder, Code Fixer):
        // treat any non-error SSE completion as success — the agent may have
        // done its work without writing the marker file.
        match &result {
            Ok(r) if r.status == ConversationStatus::Completed => ConversationStatus::Completed,
            Ok(_) => {
                eprintln!(
                    "[pipeline] {stage_name}: SSE finished but without Completed status; \
                     treating as Completed because file_required=false"
                );
                ConversationStatus::Completed
            }
            Err(e) => {
                eprintln!(
                    "[pipeline] {stage_name}: SSE stream error ({e}); \
                     treating as Completed because file_required=false"
                );
                ConversationStatus::Completed
            }
        }
    } else {
        match &result {
            Ok(r) => r.status.clone(),
            Err(_) => ConversationStatus::Failed,
        }
    };

    // When the marker file is optional and wasn't written by the agent,
    // auto-generate a fallback so the pipeline can proceed and the file
    // is available for hydration on reload.
    let file_text = if file_exists {
        std::fs::read_to_string(&file_to_watch).ok()
    } else if !file_required && final_status == ConversationStatus::Completed {
        let accumulated = output_buffer
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();

        let fallback = if accumulated.trim().is_empty() {
            format!(
                "# {stage_name} — auto-generated summary\n\n\
                 The {stage_name} stage completed but did not write a summary file.\n\
                 The agent may have performed its work without producing explicit output."
            )
        } else {
            format!(
                "# {stage_name} — auto-generated summary\n\n\
                 The {stage_name} stage completed but did not write a summary file. \
                 Below is the captured output from the session.\n\n---\n\n{accumulated}"
            )
        };

        // Best-effort write — don't fail the stage if the write fails.
        if let Some(parent) = Path::new(&file_to_watch).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&file_to_watch, &fallback) {
            Ok(()) => {
                eprintln!(
                    "[pipeline] {stage_name}: wrote fallback marker file to {file_to_watch}"
                );
                Some(fallback)
            }
            Err(e) => {
                eprintln!(
                    "[pipeline] {stage_name}: failed to write fallback marker file: {e}"
                );
                Some(fallback)
            }
        }
    } else {
        None
    };

    // Build the persisted record.
    let captured_session_ref = session_ref.lock().ok().and_then(|g| g.clone());
    let captured_job = score_id_slot.lock().ok().and_then(|g| g.clone());
    let accumulated_text = output_buffer
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();

    // Prefer the marker file content; fall back to accumulated SSE output.
    let display_text = file_text.or_else(|| {
        if accumulated_text.is_empty() {
            None
        } else {
            Some(accumulated_text.clone())
        }
    });

    let _ = emit_stage_status(
        &app, &conversation_id, stage_index, &stage_name,
        final_status.clone(), &agent_label, display_text.clone(),
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

    if let Err(e) = crate::conversations::persistence::update_pipeline_stage(
        &workspace_path,
        &conversation_id,
        &record,
    ) {
        eprintln!("[pipeline] Failed to save stage state for {stage_name}: {e}");
    }

    if final_status == ConversationStatus::Failed {
        Err((record, failure_message))
    } else {
        Ok(record)
    }
}
