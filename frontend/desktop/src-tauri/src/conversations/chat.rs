use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::commands::api_health::hive_api_base_url;
use crate::models::{
    ConversationDetail, ConversationOutputDelta, ConversationStatus, ConversationStatusEvent,
};

use super::events::{EVENT_CONVERSATION_OUTPUT_DELTA, EVENT_CONVERSATION_STATUS};
use super::persistence;
use super::sse::{consume_hive_sse, HiveSseEvent, SseResult};

fn shared_hive_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("failed to build hive client")
    })
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HiveChatRequest<'a> {
    provider: &'a str,
    model: &'a str,
    workspace_path: &'a str,
    mode: &'a str,
    prompt: &'a str,
    provider_session_ref: Option<&'a str>,
    stream: bool,
    provider_options: HashMap<String, String>,
}

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
    let request = HiveChatRequest {
        provider: &summary.agent.provider,
        model: &summary.agent.model,
        workspace_path: &summary.workspace_path,
        mode,
        prompt: &prompt,
        provider_session_ref: summary.last_provider_session_ref.as_deref(),
        stream: true,
        provider_options: HashMap::new(),
    };

    let url = format!("{}/v1/chat", hive_api_base_url());
    let response = match shared_hive_client().post(url).json(&request).send().await {
        Ok(response) => response,
        Err(error) => {
            return finish_with_failure(
                &app,
                &workspace_path,
                &conversation_id,
                None,
                format!("Failed to contact hive-api: {error}"),
            )
            .await;
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return finish_with_failure(
            &app,
            &workspace_path,
            &conversation_id,
            None,
            format!("hive-api returned HTTP {status}: {body}"),
        )
        .await;
    }

    let mut streamed_text = String::new();
    let stream_result = consume_hive_sse(response, &abort, |event| match event {
        HiveSseEvent::RunStarted { job_id } => {
            let summary = persistence::set_active_job_id(
                &workspace_path,
                &conversation_id,
                Some(job_id.clone()),
            )?;
            emit_status(
                &app,
                ConversationStatusEvent {
                    conversation: summary,
                    message: None,
                },
            )
        }
        HiveSseEvent::ProviderSession {
            provider_session_ref,
        } => {
            let summary = persistence::set_provider_session_ref(
                &workspace_path,
                &conversation_id,
                provider_session_ref.clone(),
            )?;
            emit_status(
                &app,
                ConversationStatusEvent {
                    conversation: summary,
                    message: None,
                },
            )
        }
        HiveSseEvent::OutputDelta { text } => {
            if !streamed_text.is_empty() {
                streamed_text.push('\n');
            }
            streamed_text.push_str(text);
            emit_output_delta(&app, &conversation_id, text)
        }
        HiveSseEvent::Completed { .. } | HiveSseEvent::Failed { .. } | HiveSseEvent::Stopped => {
            Ok(())
        }
    })
    .await;

    let aborted = abort.load(Ordering::Acquire);

    let result = match stream_result {
        Ok(result) => result,
        Err(error) => {
            if aborted {
                SseResult {
                    final_text: streamed_text.clone(),
                    provider_session_ref: None,
                    exit_code: None,
                    status: ConversationStatus::Stopped,
                    error: None,
                    job_id: None,
                }
            } else {
                return finish_with_failure(
                    &app,
                    &workspace_path,
                    &conversation_id,
                    if streamed_text.trim().is_empty() {
                        None
                    } else {
                        Some(streamed_text)
                    },
                    error,
                )
                .await;
            }
        }
    };

    let final_status = if aborted {
        ConversationStatus::Stopped
    } else {
        result.status
    };

    let (updated_summary, message) = persistence::finish_turn(
        &workspace_path,
        &conversation_id,
        final_status,
        if result.final_text.trim().is_empty() {
            None
        } else {
            Some(result.final_text)
        },
        result.provider_session_ref,
        result.error,
    )?;

    emit_status(
        &app,
        ConversationStatusEvent {
            conversation: updated_summary,
            message,
        },
    )
}
