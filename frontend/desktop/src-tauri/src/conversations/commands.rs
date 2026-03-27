use serde::Deserialize;
use tauri::AppHandle;
use tokio::time::{sleep, Duration, Instant};

use crate::commands::api_health::hive_api_base_url;
use crate::models::{AgentSelection, ConversationDetail, ConversationStatus, ConversationSummary};

use super::chat;
use super::persistence;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    status: ConversationStatus,
}

const STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(100);

async fn wait_for_stoppable_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    let deadline = Instant::now() + STOP_WAIT_TIMEOUT;

    loop {
        let summary = persistence::get_conversation(workspace_path, conversation_id)?.summary;
        if summary.active_job_id.is_some() || summary.status != ConversationStatus::Running {
            return Ok(summary);
        }
        if Instant::now() >= deadline {
            return Ok(summary);
        }

        sleep(STOP_WAIT_POLL_INTERVAL).await;
    }
}

#[tauri::command]
pub async fn list_workspace_conversations(workspace_path: String) -> Result<Vec<ConversationSummary>, String> {
    persistence::list_conversations(&workspace_path)
}

#[tauri::command]
pub async fn create_conversation(
    workspace_path: String,
    agent: AgentSelection,
    initial_prompt: Option<String>,
) -> Result<ConversationDetail, String> {
    persistence::create_conversation(&workspace_path, agent, initial_prompt.as_deref())
}

#[tauri::command]
pub async fn get_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationDetail, String> {
    persistence::get_conversation(&workspace_path, &conversation_id)
}

#[tauri::command]
pub async fn send_conversation_turn(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    prompt: String,
) -> Result<ConversationDetail, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Prompt must not be empty".to_string());
    }

    let detail = persistence::mark_turn_running(&workspace_path, &conversation_id, trimmed)?;
    let app_handle = app.clone();
    let detail_for_task = detail.clone();
    let prompt_for_task = trimmed.to_string();
    let tracked_workspace_path = detail.summary.workspace_path.clone();
    let tracked_conversation_id = detail.summary.id.clone();

    tokio::spawn(async move {
        let _running_guard = match persistence::track_running_conversation(
            &tracked_workspace_path,
            &tracked_conversation_id,
        ) {
            Ok(guard) => guard,
            Err(error) => {
                eprintln!("[conversation] Failed to track running conversation: {error}");
                return;
            }
        };

        if let Err(error) = chat::run_conversation_turn(app_handle, detail_for_task, prompt_for_task).await {
            eprintln!("[conversation] Failed to run conversation turn: {error}");
        }
    });

    Ok(detail)
}

#[tauri::command]
pub async fn stop_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    let mut summary = persistence::get_conversation(&workspace_path, &conversation_id)?.summary;
    if summary.status == ConversationStatus::Running && summary.active_job_id.is_none() {
        summary = wait_for_stoppable_conversation(&workspace_path, &conversation_id).await?;
    }

    let Some(job_id) = summary.active_job_id.clone() else {
        if summary.status == ConversationStatus::Running {
            return Err("The run is still starting and cannot be stopped yet. Please try again in a moment.".to_string());
        }
        return Ok(summary);
    };

    let url = format!("{}/v1/chat/{job_id}/stop", hive_api_base_url());
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|error| format!("Failed to stop conversation: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to stop conversation: HTTP {status} — {body}"));
    }

    let _stop_response = response
        .json::<StopResponse>()
        .await
        .map_err(|error| format!("Failed to parse stop response: {error}"))?;

    if _stop_response.status == ConversationStatus::Stopped {
        return persistence::set_status(
            &workspace_path,
            &conversation_id,
            ConversationStatus::Stopped,
            None,
        );
    }

    persistence::get_conversation(&workspace_path, &conversation_id).map(|detail| detail.summary)
}

#[tauri::command]
pub async fn delete_conversation(workspace_path: String, conversation_id: String) -> Result<(), String> {
    persistence::delete_conversation(&workspace_path, &conversation_id)
}
