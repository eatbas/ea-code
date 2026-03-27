use serde::Deserialize;
use tauri::AppHandle;

use crate::commands::api_health::hive_api_base_url;
use crate::models::{AgentSelection, ConversationDetail, ConversationSummary};

use super::chat;
use super::persistence;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    #[allow(dead_code)]
    status: String,
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

    tokio::spawn(async move {
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
    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    let Some(job_id) = detail.summary.active_job_id.clone() else {
        return Ok(detail.summary);
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

    persistence::get_conversation(&workspace_path, &conversation_id).map(|detail| detail.summary)
}

#[tauri::command]
pub async fn delete_conversation(workspace_path: String, conversation_id: String) -> Result<(), String> {
    persistence::delete_conversation(&workspace_path, &conversation_id)
}
