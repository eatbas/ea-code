//! Continue the Coder agent with a follow-up prompt after the pipeline has
//! finished. Reuses the most recent coder-chain Symphony session so the agent
//! retains the full context of the completed run.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use tauri::AppHandle;

use crate::conversations::pipeline::stage_runner::{run_stage, StageConfig};
use crate::conversations::pipeline_debug::emit_pipeline_debug;
use crate::models::{ConversationDetail, ConversationStatus, PipelineStageRecord};
use crate::storage::now_rfc3339;

use super::super::super::persistence;
use super::super::pipeline_orchestration::{
    begin_pipeline_task, emit_final_status, load_pipeline_config, pipeline_cleanup,
    prepare_pipeline_with_config,
};

/// Starting stage_index for post-pipeline follow-up turns. Kept well above any
/// regular pipeline stage index so follow-ups cannot collide with stages
/// added by other pipeline commands.
const FOLLOW_UP_STAGE_BASE: usize = 10_000;

/// Resume the Coder session with a follow-up prompt. Appends a new
/// `Follow-up N` stage to the pipeline state and runs it in resume mode.
#[tauri::command]
pub async fn continue_coder(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    prompt: String,
) -> Result<ConversationDetail, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Prompt must not be empty".to_string());
    }

    let state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    let session_ref = latest_coder_session_ref(&state.stages).ok_or(
        "No coder session to continue \u{2014} start a new pipeline instead.",
    )?;

    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    if detail.summary.status == ConversationStatus::Running {
        return Err("Conversation is still running".to_string());
    }

    let config = load_pipeline_config()?;
    let coder_agent = config.coder.clone();
    let agent_label = format!("{} / {}", coder_agent.provider, coder_agent.model);

    // Reuse pipeline runtime registries so Stop/abort still works and this
    // follow-up run shares the conversation-level abort flag.
    let setup = prepare_pipeline_with_config(&workspace_path, &conversation_id, config)?;

    // Allocate the follow-up's stage identity. Indices use a high base value
    // so they cannot collide with regular pipeline stages even on sparse
    // re-do cycles.
    let follow_up_count = state
        .stages
        .iter()
        .filter(|s| s.stage_name.starts_with("Follow-up"))
        .count();
    let stage_name = format!("Follow-up {}", follow_up_count + 1);
    let stage_index = FOLLOW_UP_STAGE_BASE + follow_up_count;
    let started_at = now_rfc3339();

    seed_follow_up_stage(
        &workspace_path,
        &conversation_id,
        stage_index,
        &stage_name,
        &agent_label,
        trimmed,
        &started_at,
    )?;

    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "Continue coder: follow-up #{} resuming session {session_ref}",
            follow_up_count + 1
        ),
    );

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let user_prompt = trimmed.to_string();

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };

        // Follow-ups run outside the fixed stage layout, so allocate a local
        // score-id slot and output buffer rather than borrowing the setup's.
        let score_id_slot = Arc::new(Mutex::new(None));
        let output_buffer = Arc::new(Mutex::new(String::new()));

        let result = run_stage(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            StageConfig {
                stage_index,
                stage_name: stage_name.clone(),
                provider: coder_agent.provider.clone(),
                model: coder_agent.model.clone(),
                prompt: user_prompt,
                // Follow-ups do not write a marker file; completion is driven
                // by the Symphony score status alone.
                file_to_watch: String::new(),
                mode: "resume",
                provider_session_ref: Some(session_ref),
                failure_message: format!("{stage_name} did not produce a response"),
                agent_label: agent_label.clone(),
                file_required: false,
            },
            setup.abort.clone(),
            score_id_slot,
            output_buffer,
        )
        .await;

        let (status, error) = if setup.abort.load(Ordering::Acquire) {
            (ConversationStatus::Stopped, None)
        } else {
            match &result {
                Ok(_) => (ConversationStatus::Completed, None),
                Err((_, error)) => (ConversationStatus::Failed, Some(error.clone())),
            }
        };

        emit_final_status(&app_handle, &ws, &conv_id, status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}

/// Return the provider session ref of the newest coder-chain stage, if any.
/// Priority (newest first): previous Follow-up turns, latest Code Fixer
/// (including re-do cycles), and finally the Coder stage itself.
fn latest_coder_session_ref(stages: &[PipelineStageRecord]) -> Option<String> {
    // Walk in reverse so the most recent chain stage wins. Follow-ups were
    // appended last, then Code Fixers from later cycles, then the original
    // Coder — this ordering matches insertion order.
    stages.iter().rev().find_map(|stage| {
        let name = stage.stage_name.as_str();
        let in_chain = name.starts_with("Follow-up")
            || name.starts_with("Code Fixer")
            || name == "Coder"
            || name.starts_with("Coder ");
        if in_chain {
            stage.provider_session_ref.clone()
        } else {
            None
        }
    })
}

/// Append a running follow-up stage to pipeline.json so the frontend can
/// render the user prompt immediately (and survive a reload).
fn seed_follow_up_stage(
    workspace_path: &str,
    conversation_id: &str,
    stage_index: usize,
    stage_name: &str,
    agent_label: &str,
    user_prompt: &str,
    started_at: &str,
) -> Result<(), String> {
    let record = PipelineStageRecord {
        stage_index,
        stage_name: stage_name.to_string(),
        agent_label: agent_label.to_string(),
        status: ConversationStatus::Running,
        text: String::new(),
        started_at: Some(started_at.to_string()),
        finished_at: None,
        score_id: None,
        provider_session_ref: None,
        user_prompt: Some(user_prompt.to_string()),
    };
    persistence::update_pipeline_stage(workspace_path, conversation_id, &record)
}
