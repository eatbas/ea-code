mod emission;
mod finalise;
mod watchers;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::AppHandle;
use tokio::time::{sleep, Duration, Instant};

use crate::conversations::pipeline_debug::emit_pipeline_debug;
use crate::conversations::score_client::{
    consume_score_websocket, fetch_score_snapshot, SymphonyLiveEvent, SCORE_POLL_INTERVAL,
};
use crate::conversations::symphony_request::SymphonyChatRequest;
use crate::models::{ConversationStatus, PipelineStageRecord};
use crate::storage::now_rfc3339;

use self::emission::{emit_stage_delta, request_symphony_stop};
use self::finalise::{
    append_live_output, describe_stage_failure, determine_final_status, live_output,
    maybe_update_session_ref, resolve_stage_text, sync_snapshot_output,
};
use self::watchers::spawn_stage_watchers;

const FILE_COMPLETION_STOP_GRACE: Duration = Duration::from_secs(15);
const POLL_FAILURE_GRACE: Duration = Duration::from_secs(45);

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
    /// When `false`, the stage completes based on the score status alone.
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

/// Execute a single pipeline stage: submit a score to Symphony, observe it via
/// polling and optional WebSocket deltas, watch for the expected plan file, and
/// persist the result.
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
    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "{stage_name}: starting stage with {provider}/{model}, mode={mode}, file_required={file_required}, file_to_watch={file_to_watch}",
        ),
    );

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

    let thinking_level = crate::storage::settings::read_settings()
        .ok()
        .and_then(|s| s.thinking_level(&provider, &model).map(str::to_string));
    let request = SymphonyChatRequest {
        provider: &provider,
        model: &model,
        workspace_path: &workspace_path,
        mode,
        prompt: &prompt,
        provider_session_ref: provider_session_ref.as_deref(),
        provider_options: crate::conversations::symphony_request::default_provider_options(
            &provider,
            thinking_level.as_deref(),
        ),
    };

    let emit_failed = |error_message: &str| -> (PipelineStageRecord, String) {
        let diagnostic = format!("# {stage_name} failed\n\n{error_message}");
        let _ = emit_stage_status(
            &app,
            &conversation_id,
            stage_index,
            &stage_name,
            ConversationStatus::Failed,
            &agent_label,
            Some(diagnostic.clone()),
        );
        let mut record = PipelineStageRecord::failed(
            stage_index,
            stage_name.clone(),
            agent_label.clone(),
            Some(started_at.clone()),
        );
        record.text = diagnostic;
        (record, error_message.to_string())
    };

    let accepted = match crate::conversations::score_client::submit_score(&request).await {
        Ok(response) => response,
        Err(error) => {
            emit_pipeline_debug(
                &app,
                &workspace_path,
                &conversation_id,
                format!("{stage_name}: failed to submit score: {error}"),
            );
            return Err(emit_failed(&format!(
                "{stage_name} failed to submit to symphony: {error}"
            )));
        }
    };
    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!("{stage_name}: accepted score {}", accepted.score_id),
    );

    if let Ok(mut guard) = score_id_slot.lock() {
        *guard = Some(accepted.score_id.clone());
    }

    let watchers = spawn_stage_watchers(file_to_watch.clone(), abort.clone());
    let app_ref = app.clone();
    let conversation_id_ref = conversation_id.clone();
    let output_buffer_writer = output_buffer.clone();
    let session_ref: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
    let session_ref_writer = session_ref.clone();
    let websocket_stop = Arc::new(AtomicBool::new(false));

    {
        let websocket_stop_ref = websocket_stop.clone();
        let score_id = accepted.score_id.clone();
        let stage_name_for_ws = stage_name.clone();
        let app_for_ws = app.clone();
        let workspace_for_ws = workspace_path.clone();
        let conversation_for_ws = conversation_id.clone();
        tokio::spawn(async move {
            let result =
                consume_score_websocket(
                    &score_id,
                    websocket_stop_ref.as_ref(),
                    |event| match event {
                        SymphonyLiveEvent::ScoreSnapshot(snapshot) => {
                            if let Some(delta) = sync_snapshot_output(
                                &output_buffer_writer,
                                &snapshot.accumulated_text,
                            ) {
                                emit_stage_delta(
                                    &app_ref,
                                    &conversation_id_ref,
                                    stage_index,
                                    &delta,
                                )?;
                            }
                            maybe_update_session_ref(
                                &session_ref_writer,
                                snapshot.provider_session_ref.as_deref(),
                            );
                            Ok(())
                        }
                        SymphonyLiveEvent::OutputDelta { text } => {
                            append_live_output(&output_buffer_writer, &text);
                            emit_stage_delta(&app_ref, &conversation_id_ref, stage_index, &text)
                        }
                        SymphonyLiveEvent::ProviderSession {
                            provider_session_ref,
                        } => {
                            maybe_update_session_ref(
                                &session_ref_writer,
                                Some(provider_session_ref.as_str()),
                            );
                            emit_pipeline_debug(
                                &app_for_ws,
                                &workspace_for_ws,
                                &conversation_for_ws,
                                format!(
                                    "{stage_name_for_ws}: provider session captured via websocket"
                                ),
                            );
                            Ok(())
                        }
                        SymphonyLiveEvent::Ignored => Ok(()),
                    },
                )
                .await;

            if let Err(error) = result {
                eprintln!("[pipeline] {stage_name_for_ws}: Symphony WebSocket error: {error}");
                emit_pipeline_debug(
                    &app_for_ws,
                    &workspace_for_ws,
                    &conversation_for_ws,
                    format!("{stage_name_for_ws}: websocket closed with error: {error}"),
                );
            }
        });
    }

    let mut stop_requested = false;
    let mut stop_requested_at: Option<Instant> = None;
    let mut poll_failure_started_at: Option<Instant> = None;
    let mut last_status: Option<String> = None;
    let terminal_snapshot = loop {
        let watched_file_exists = Path::new(&file_to_watch).exists();
        if (watchers.local_stop.load(Ordering::Acquire)
            || abort.load(Ordering::Acquire)
            || watched_file_exists)
            && !stop_requested
        {
            let stop_reason = if abort.load(Ordering::Acquire) {
                "abort flag set"
            } else if watched_file_exists {
                "watched file exists"
            } else {
                "watched file became ready"
            };
            emit_pipeline_debug(
                &app,
                &workspace_path,
                &conversation_id,
                format!(
                    "{stage_name}: issuing stop request for score {} because {stop_reason}",
                    accepted.score_id
                ),
            );
            request_symphony_stop(accepted.score_id.clone());
            stop_requested = true;
            stop_requested_at = Some(Instant::now());
        }

        match fetch_score_snapshot(&accepted.score_id).await {
            Ok(snapshot) => {
                if let Some(failure_started_at) = poll_failure_started_at.take() {
                    emit_pipeline_debug(
                        &app,
                        &workspace_path,
                        &conversation_id,
                        format!(
                            "{stage_name}: polling recovered after {:.1}s",
                            failure_started_at.elapsed().as_secs_f32(),
                        ),
                    );
                }
                let status_label = format!("{:?}", snapshot.status).to_lowercase();
                if last_status.as_deref() != Some(status_label.as_str()) {
                    emit_pipeline_debug(
                        &app,
                        &workspace_path,
                        &conversation_id,
                        format!(
                            "{stage_name}: polled status -> {status_label} (chars={}, final={}, error={})",
                            snapshot.accumulated_text.len(),
                            snapshot.final_text.as_ref().map(|text| !text.is_empty()).unwrap_or(false),
                            snapshot.error.as_deref().unwrap_or("none"),
                        ),
                    );
                    last_status = Some(status_label);
                }
                if let Some(delta) =
                    sync_snapshot_output(&output_buffer, &snapshot.accumulated_text)
                {
                    emit_stage_delta(&app, &conversation_id, stage_index, &delta)
                        .map_err(|error| emit_failed(&error))?;
                }
                maybe_update_session_ref(&session_ref, snapshot.provider_session_ref.as_deref());
                if snapshot.status.is_terminal() {
                    emit_pipeline_debug(
                        &app,
                        &workspace_path,
                        &conversation_id,
                        format!("{stage_name}: reached terminal snapshot state"),
                    );
                    break Ok(snapshot);
                }
                if stop_requested
                    && Path::new(&file_to_watch).exists()
                    && stop_requested_at
                        .map(|instant| instant.elapsed() >= FILE_COMPLETION_STOP_GRACE)
                        .unwrap_or(false)
                {
                    emit_pipeline_debug(
                        &app,
                        &workspace_path,
                        &conversation_id,
                        format!(
                            "{stage_name}: watched file exists but score {} stayed non-terminal after stop grace; finalising from file",
                            accepted.score_id,
                        ),
                    );
                    break Err(format!(
                        "{stage_name} stop grace expired while waiting for Symphony score {} to terminate",
                        accepted.score_id,
                    ));
                }
            }
            Err(error) => {
                let first_failure = poll_failure_started_at.is_none();
                let failure_started_at = poll_failure_started_at.get_or_insert_with(Instant::now);
                if Path::new(&file_to_watch).exists() {
                    if !stop_requested {
                        emit_pipeline_debug(
                            &app,
                            &workspace_path,
                            &conversation_id,
                            format!(
                                "{stage_name}: polling failed after watched file appeared; issuing best-effort stop for score {}",
                                accepted.score_id,
                            ),
                        );
                        request_symphony_stop(accepted.score_id.clone());
                    }
                    emit_pipeline_debug(
                        &app,
                        &workspace_path,
                        &conversation_id,
                        format!(
                            "{stage_name}: polling failed after watched file existed; finalising from file: {error}",
                        ),
                    );
                    break Err(error);
                }
                if failure_started_at.elapsed() < POLL_FAILURE_GRACE {
                    if first_failure {
                        emit_pipeline_debug(
                            &app,
                            &workspace_path,
                            &conversation_id,
                            format!(
                                "{stage_name}: polling failed but retrying for up to {}s: {error}",
                                POLL_FAILURE_GRACE.as_secs(),
                            ),
                        );
                    }
                    sleep(SCORE_POLL_INTERVAL).await;
                    continue;
                }
                emit_pipeline_debug(
                    &app,
                    &workspace_path,
                    &conversation_id,
                    format!(
                        "{stage_name}: polling failed after {}s grace: {error}",
                        POLL_FAILURE_GRACE.as_secs(),
                    ),
                );
                break Err(error);
            }
        }

        sleep(SCORE_POLL_INTERVAL).await;
    };

    websocket_stop.store(true, Ordering::Release);

    let final_status = determine_final_status(
        abort.as_ref(),
        &file_to_watch,
        watchers.file_ready.as_ref(),
        file_required,
        &stage_name,
        &terminal_snapshot,
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
    let accumulated_text = live_output(&output_buffer);
    let terminal_text = terminal_snapshot
        .as_ref()
        .ok()
        .and_then(|snapshot| snapshot.final_text.clone())
        .filter(|text| !text.trim().is_empty());
    let failure_text = if final_status == ConversationStatus::Failed {
        Some(describe_stage_failure(
            &stage_name,
            &file_to_watch,
            file_required,
            &terminal_snapshot,
            captured_job.as_deref(),
            captured_session_ref.as_deref(),
            &accumulated_text,
        ))
    } else {
        None
    };
    let display_text = file_text
        .or(terminal_text)
        .or_else(|| {
            if accumulated_text.is_empty() {
                None
            } else {
                Some(accumulated_text.clone())
            }
        })
        .or(failure_text.clone());

    if let Some(ref diagnostic) = failure_text {
        eprintln!("[pipeline] {stage_name}: {diagnostic}");
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("{stage_name}: failure diagnostic recorded"),
        );
    }

    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "{stage_name}: final status resolved to {:?}; watched_file_exists={}",
            final_status,
            std::path::Path::new(&file_to_watch).exists(),
        ),
    );

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
