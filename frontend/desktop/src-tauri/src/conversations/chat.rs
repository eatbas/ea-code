use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::models::{
    ConversationDetail, ConversationOutputDelta, ConversationStatus, ConversationStatusEvent,
};

use super::events::{EVENT_CONVERSATION_OUTPUT_DELTA, EVENT_CONVERSATION_STATUS};
use super::persistence;
use super::score_client::{
    consume_score_websocket, poll_until_terminal, submit_score, SymphonyLiveEvent,
    SymphonyScoreSnapshot,
};
use super::symphony_request::{default_provider_options, SymphonyChatRequest};

fn emit_status(app: &AppHandle, event: ConversationStatusEvent) -> Result<(), String> {
    app.emit(EVENT_CONVERSATION_STATUS, event)
        .map_err(|error| format!("Failed to emit conversation status: {error}"))
}

fn emit_output_delta(app: &AppHandle, conversation_id: &str, text: &str) -> Result<(), String> {
    app.emit(
        EVENT_CONVERSATION_OUTPUT_DELTA,
        ConversationOutputDelta {
            conversation_id: conversation_id.to_string(),
            text: text.to_string(),
        },
    )
    .map_err(|error| format!("Failed to emit conversation output delta: {error}"))
}

fn append_live_output(buffer: &Arc<Mutex<String>>, text: &str) {
    if let Ok(mut guard) = buffer.lock() {
        if !guard.is_empty() {
            guard.push('\n');
        }
        guard.push_str(text);
    }
}

fn sync_snapshot_output(buffer: &Arc<Mutex<String>>, accumulated_text: &str) -> Option<String> {
    let Ok(mut guard) = buffer.lock() else {
        return None;
    };

    if accumulated_text.starts_with(guard.as_str()) {
        let suffix = accumulated_text[guard.len()..]
            .trim_start_matches('\n')
            .to_string();
        *guard = accumulated_text.to_string();
        return (!suffix.is_empty()).then_some(suffix);
    }

    *guard = accumulated_text.to_string();
    None
}

fn live_output(buffer: &Arc<Mutex<String>>) -> String {
    buffer.lock().map(|guard| guard.clone()).unwrap_or_default()
}

fn maybe_update_provider_session(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    session_slot: &Arc<Mutex<Option<String>>>,
    next_session: Option<&str>,
) -> Result<(), String> {
    let Some(next_session) = next_session.map(str::to_string) else {
        return Ok(());
    };

    let Ok(mut guard) = session_slot.lock() else {
        return Ok(());
    };
    if guard.as_deref() == Some(next_session.as_str()) {
        return Ok(());
    }

    *guard = Some(next_session.clone());
    let summary =
        persistence::set_provider_session_ref(workspace_path, conversation_id, next_session)?;
    emit_status(
        app,
        ConversationStatusEvent {
            conversation: summary,
            message: None,
        },
    )
}

async fn finish_with_failure(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    assistant_text: Option<String>,
    error: String,
) -> Result<(), String> {
    let (summary, message) = persistence::finish_turn(
        workspace_path,
        conversation_id,
        ConversationStatus::Failed,
        assistant_text,
        None,
        Some(error),
    )?;
    emit_status(
        app,
        ConversationStatusEvent {
            conversation: summary,
            message,
        },
    )
}

pub async fn run_conversation_turn(
    app: AppHandle,
    detail: ConversationDetail,
    prompt: String,
    abort: Arc<AtomicBool>,
) -> Result<(), String> {
    let summary = detail.summary;
    let workspace_path = summary.workspace_path.clone();
    let conversation_id = summary.id.clone();
    let mode = if summary.last_provider_session_ref.is_some() {
        "resume"
    } else {
        "new"
    };
    let thinking_level = crate::storage::settings::read_settings()
        .ok()
        .and_then(|s| s.thinking_level_for_provider(&summary.agent.provider).map(str::to_string));
    let request = SymphonyChatRequest {
        provider: &summary.agent.provider,
        model: &summary.agent.model,
        workspace_path: &summary.workspace_path,
        mode,
        prompt: &prompt,
        provider_session_ref: summary.last_provider_session_ref.as_deref(),
        provider_options: default_provider_options(
            &summary.agent.provider,
            thinking_level.as_deref(),
        ),
    };

    let accepted = match submit_score(&request).await {
        Ok(response) => response,
        Err(error) => {
            return finish_with_failure(&app, &workspace_path, &conversation_id, None, error).await;
        }
    };

    let updated_summary = persistence::set_active_score_id(
        &workspace_path,
        &conversation_id,
        Some(accepted.score_id.clone()),
    )?;
    emit_status(
        &app,
        ConversationStatusEvent {
            conversation: updated_summary,
            message: None,
        },
    )?;

    let live_buffer = Arc::new(Mutex::new(String::new()));
    let provider_session_ref = Arc::new(Mutex::new(summary.last_provider_session_ref.clone()));
    let websocket_stop = Arc::new(AtomicBool::new(false));

    {
        let app_ref = app.clone();
        let workspace_ref = workspace_path.clone();
        let conversation_ref = conversation_id.clone();
        let score_id = accepted.score_id.clone();
        let live_buffer_ref = live_buffer.clone();
        let session_ref = provider_session_ref.clone();
        let websocket_stop_ref = websocket_stop.clone();

        tokio::spawn(async move {
            let result =
                consume_score_websocket(
                    &score_id,
                    websocket_stop_ref.as_ref(),
                    |event| match event {
                        SymphonyLiveEvent::ScoreSnapshot(snapshot) => {
                            if let Some(delta) =
                                sync_snapshot_output(&live_buffer_ref, &snapshot.accumulated_text)
                            {
                                emit_output_delta(&app_ref, &conversation_ref, &delta)?;
                            }
                            maybe_update_provider_session(
                                &app_ref,
                                &workspace_ref,
                                &conversation_ref,
                                &session_ref,
                                snapshot.provider_session_ref.as_deref(),
                            )
                        }
                        SymphonyLiveEvent::OutputDelta { text } => {
                            append_live_output(&live_buffer_ref, &text);
                            emit_output_delta(&app_ref, &conversation_ref, &text)
                        }
                        SymphonyLiveEvent::ProviderSession {
                            provider_session_ref,
                        } => maybe_update_provider_session(
                            &app_ref,
                            &workspace_ref,
                            &conversation_ref,
                            &session_ref,
                            Some(provider_session_ref.as_str()),
                        ),
                        SymphonyLiveEvent::Ignored => Ok(()),
                    },
                )
                .await;

            if let Err(error) = result {
                eprintln!("[conversation] Symphony WebSocket closed with error: {error}");
            }
        });
    }

    let terminal_snapshot =
        poll_until_terminal(accepted.score_id.as_str(), abort.as_ref(), |snapshot| {
            if let Some(delta) = sync_snapshot_output(&live_buffer, &snapshot.accumulated_text) {
                emit_output_delta(&app, &conversation_id, &delta)?;
            }
            maybe_update_provider_session(
                &app,
                &workspace_path,
                &conversation_id,
                &provider_session_ref,
                snapshot.provider_session_ref.as_deref(),
            )
        })
        .await;

    websocket_stop.store(true, Ordering::Release);

    let snapshot = match terminal_snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let partial = live_output(&live_buffer);
            return finish_with_failure(
                &app,
                &workspace_path,
                &conversation_id,
                (!partial.trim().is_empty()).then_some(partial),
                error,
            )
            .await;
        }
    };

    finish_conversation_from_snapshot(
        &app,
        &workspace_path,
        &conversation_id,
        &live_buffer,
        snapshot,
    )
}

fn finish_conversation_from_snapshot(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    live_buffer: &Arc<Mutex<String>>,
    snapshot: SymphonyScoreSnapshot,
) -> Result<(), String> {
    let assistant_text = snapshot
        .final_text
        .clone()
        .or_else(|| {
            (!snapshot.accumulated_text.trim().is_empty())
                .then(|| snapshot.accumulated_text.clone())
        })
        .or_else(|| {
            let live = live_output(live_buffer);
            (!live.trim().is_empty()).then_some(live)
        });

    let (updated_summary, message) = persistence::finish_turn(
        workspace_path,
        conversation_id,
        snapshot.status.as_conversation_status(),
        assistant_text,
        snapshot.provider_session_ref.clone(),
        snapshot.error.clone(),
    )?;

    emit_status(
        app,
        ConversationStatusEvent {
            conversation: updated_summary,
            message,
        },
    )
}
