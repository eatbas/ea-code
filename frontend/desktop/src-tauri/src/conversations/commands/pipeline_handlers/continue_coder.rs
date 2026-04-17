//! Continue the Coder agent with a follow-up prompt after the pipeline has
//! finished. The follow-up turn reuses the most recent coder-chain Symphony
//! session so the agent retains full context, but its prompt and reply are
//! persisted as ordinary chat messages rather than pipeline stage records.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::AppHandle;

use crate::conversations::chat::{
    append_live_output, emit_output_delta, emit_status, live_output, maybe_update_provider_session,
    sync_snapshot_output,
};
use crate::conversations::pipeline_debug::emit_pipeline_debug;
use crate::conversations::score_client::{
    consume_score_websocket, poll_until_terminal, submit_score, SymphonyLiveEvent,
    SymphonyScoreSnapshot,
};
use crate::conversations::symphony_request::{default_provider_options, SymphonyChatRequest};
use crate::models::{
    AgentSelection, ConversationDetail, ConversationStatus, ConversationStatusEvent,
    PipelineStageRecord,
};

use super::super::super::persistence;
use super::super::pipeline_orchestration::load_pipeline_config;

/// Resume the Coder session with a follow-up prompt. The prompt is appended
/// as a `User` message and the assistant's reply streams into an `Assistant`
/// message tagged with the coder agent.
#[tauri::command]
pub async fn continue_coder(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    prompt: String,
) -> Result<ConversationDetail, String> {
    let trimmed = prompt.trim().to_string();
    if trimmed.is_empty() {
        return Err("Prompt must not be empty".to_string());
    }

    let state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    // Prefer the summary's last coder session (set by the previous follow-up)
    // before falling back to the pipeline's coder-chain stages so we always
    // resume the newest agent session.
    let summary_snapshot = persistence::get_conversation(&workspace_path, &conversation_id)?;
    if summary_snapshot.summary.status == ConversationStatus::Running {
        return Err("Conversation is still running".to_string());
    }
    let session_ref = summary_snapshot
        .summary
        .last_provider_session_ref
        .clone()
        .or_else(|| latest_coder_session_ref(&state.stages))
        .ok_or("No coder session to continue \u{2014} start a new pipeline instead.")?;

    let config = load_pipeline_config()?;
    let coder_agent = AgentSelection {
        provider: config.coder.provider.clone(),
        model: config.coder.model.clone(),
    };

    let detail = persistence::mark_turn_running(&workspace_path, &conversation_id, &trimmed)?;
    emit_status(
        &app,
        ConversationStatusEvent {
            conversation: detail.summary.clone(),
            message: detail.messages.last().cloned(),
        },
    )?;

    let abort = persistence::register_abort_flag(&workspace_path, &conversation_id)?;
    let guard = persistence::track_running_conversation(&workspace_path, &conversation_id)?;

    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!("Continue coder: resuming session {session_ref}"),
    );

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();

    tokio::spawn(async move {
        // Move the guard into the task so it stays alive for the whole run
        // and drops on completion — that clears the in-memory and persisted
        // running flags.
        let _guard = guard;
        let outcome = run_follow_up_turn(
            &app_handle,
            &ws,
            &conv_id,
            trimmed,
            coder_agent.clone(),
            session_ref,
            abort.clone(),
        )
        .await;

        persistence::remove_abort_flag(&ws, &conv_id).ok();

        if let Err(error) = outcome {
            eprintln!("[continue_coder] follow-up turn failed: {error}");
        }
    });

    Ok(detail)
}

async fn run_follow_up_turn(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    prompt: String,
    coder_agent: AgentSelection,
    session_ref: String,
    abort: Arc<AtomicBool>,
) -> Result<(), String> {
    let thinking_level = crate::storage::settings::read_settings()
        .ok()
        .and_then(|s| {
            s.thinking_level(&coder_agent.provider, &coder_agent.model)
                .map(str::to_string)
        });
    let provider_options =
        default_provider_options(&coder_agent.provider, thinking_level.as_deref(), None);

    emit_pipeline_debug(
        app,
        workspace_path,
        conversation_id,
        format!(
            "follow-up submit: provider={} model={} mode=resume session_ref={}",
            coder_agent.provider, coder_agent.model, session_ref,
        ),
    );

    let request = SymphonyChatRequest {
        provider: &coder_agent.provider,
        model: &coder_agent.model,
        workspace_path,
        mode: "resume",
        prompt: &prompt,
        provider_session_ref: Some(session_ref.as_str()),
        provider_options,
    };

    let accepted = match submit_score(&request).await {
        Ok(response) => response,
        Err(error) => {
            return finalise_follow_up(
                app,
                workspace_path,
                conversation_id,
                ConversationStatus::Failed,
                None,
                None,
                Some(error),
                &coder_agent,
            );
        }
    };
    emit_pipeline_debug(
        app,
        workspace_path,
        conversation_id,
        format!("follow-up: accepted score {}", accepted.score_id),
    );

    let score_summary = persistence::set_active_score_id(
        workspace_path,
        conversation_id,
        Some(accepted.score_id.clone()),
    )?;
    emit_status(
        app,
        ConversationStatusEvent {
            conversation: score_summary,
            message: None,
        },
    )?;

    let live_buffer = Arc::new(Mutex::new(String::new()));
    let provider_session_ref = Arc::new(Mutex::new(Some(session_ref.clone())));
    let websocket_stop = Arc::new(AtomicBool::new(false));

    {
        let app_ref = app.clone();
        let workspace_ref = workspace_path.to_string();
        let conversation_ref = conversation_id.to_string();
        let score_id = accepted.score_id.clone();
        let live_buffer_ref = live_buffer.clone();
        let session_ref_cell = provider_session_ref.clone();
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
                                &session_ref_cell,
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
                            &session_ref_cell,
                            Some(provider_session_ref.as_str()),
                        ),
                        SymphonyLiveEvent::Ignored => Ok(()),
                    },
                )
                .await;

            if let Err(error) = result {
                eprintln!("[continue_coder] Symphony WebSocket closed with error: {error}");
            }
        });
    }

    let terminal_snapshot =
        poll_until_terminal(accepted.score_id.as_str(), abort.as_ref(), |snapshot| {
            if let Some(delta) = sync_snapshot_output(&live_buffer, &snapshot.accumulated_text) {
                emit_output_delta(app, conversation_id, &delta)?;
            }
            maybe_update_provider_session(
                app,
                workspace_path,
                conversation_id,
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
            return finalise_follow_up(
                app,
                workspace_path,
                conversation_id,
                ConversationStatus::Failed,
                (!partial.trim().is_empty()).then_some(partial),
                None,
                Some(error),
                &coder_agent,
            );
        }
    };

    let status = if abort.load(Ordering::Acquire) {
        ConversationStatus::Stopped
    } else {
        snapshot.status.as_conversation_status()
    };

    finalise_from_snapshot(
        app,
        workspace_path,
        conversation_id,
        &live_buffer,
        snapshot,
        status,
        &coder_agent,
    )
}

fn finalise_from_snapshot(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    live_buffer: &Arc<Mutex<String>>,
    snapshot: SymphonyScoreSnapshot,
    status: ConversationStatus,
    coder_agent: &AgentSelection,
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

    finalise_follow_up(
        app,
        workspace_path,
        conversation_id,
        status,
        assistant_text,
        snapshot.provider_session_ref.clone(),
        snapshot.error.clone(),
        coder_agent,
    )
}

fn finalise_follow_up(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    status: ConversationStatus,
    assistant_text: Option<String>,
    provider_session_ref: Option<String>,
    error: Option<String>,
    coder_agent: &AgentSelection,
) -> Result<(), String> {
    let (summary, message) = persistence::finish_turn_with_message_agent(
        workspace_path,
        conversation_id,
        status,
        assistant_text,
        provider_session_ref,
        error,
        coder_agent,
    )?;

    emit_status(
        app,
        ConversationStatusEvent {
            conversation: summary,
            message,
        },
    )
}

/// Return the provider session ref of the newest coder-chain stage, if any.
/// Walks stages in reverse so the most recent in-chain stage wins (latest
/// Code Fixer from re-do cycles, then the original Coder).
fn latest_coder_session_ref(stages: &[PipelineStageRecord]) -> Option<String> {
    stages.iter().rev().find_map(|stage| {
        let name = stage.stage_name.as_str();
        let in_chain =
            name.starts_with("Code Fixer") || name == "Coder" || name.starts_with("Coder ");
        if in_chain {
            stage.provider_session_ref.clone()
        } else {
            None
        }
    })
}
