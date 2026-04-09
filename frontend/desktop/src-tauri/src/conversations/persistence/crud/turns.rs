//! Turn and status mutation operations.

use crate::models::{
    AgentSelection, ConversationDetail, ConversationMessage, ConversationMessageRole,
    ConversationStatus, ConversationSummary,
};
use crate::storage::{now_rfc3339, with_conversations_lock};

use super::super::io::{
    read_messages_unlocked, read_summary_unlocked, write_messages_unlocked, write_summary_unlocked,
};
use super::super::recovery::{normalise_title, reconcile_stale_running_unlocked};

pub fn mark_turn_running(
    workspace_path: &str,
    conversation_id: &str,
    prompt: &str,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        reconcile_stale_running_unlocked(&mut summary)?;
        if summary.status == ConversationStatus::Running {
            return Err("This conversation is already running".to_string());
        }

        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;
        let user_message = ConversationMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: ConversationMessageRole::User,
            content: prompt.to_string(),
            created_at: now_rfc3339(),
            agent: None,
            thinking_level: None,
        };
        messages.push(user_message);
        summary.message_count = messages.len();
        if summary.message_count == 1 {
            summary.title = normalise_title(prompt);
        }
        summary.status = ConversationStatus::Running;
        summary.updated_at = now_rfc3339();
        summary.active_score_id = None;
        summary.error = None;

        write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        write_summary_unlocked(&summary)?;

        Ok(ConversationDetail { summary, messages })
    })
}

pub fn set_active_score_id(
    workspace_path: &str,
    conversation_id: &str,
    score_id: Option<String>,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.active_score_id = score_id;
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn set_provider_session_ref(
    workspace_path: &str,
    conversation_id: &str,
    provider_session_ref: String,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.last_provider_session_ref = Some(provider_session_ref);
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn set_status(
    workspace_path: &str,
    conversation_id: &str,
    status: ConversationStatus,
    error: Option<String>,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.status = status;
        summary.updated_at = now_rfc3339();
        summary.active_score_id = None;
        summary.error = error;
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn finish_turn(
    workspace_path: &str,
    conversation_id: &str,
    status: ConversationStatus,
    assistant_text: Option<String>,
    provider_session_ref: Option<String>,
    error: Option<String>,
    model_override: Option<&str>,
) -> Result<(ConversationSummary, Option<ConversationMessage>), String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;

        // Commit the model override only on success — if the CLI rejects
        // the model switch the conversation keeps its original model.
        let effective_agent = if status == ConversationStatus::Completed {
            if let Some(new_model) = model_override {
                summary.agent.model = new_model.to_string();
            }
            summary.agent.clone()
        } else {
            // For the per-message label, show which model was *attempted*
            // even if the turn failed.
            match model_override {
                Some(m) => AgentSelection {
                    provider: summary.agent.provider.clone(),
                    model: m.to_string(),
                },
                None => summary.agent.clone(),
            }
        };

        // Read the thinking level from settings so it can be stored per-message.
        let thinking_level = crate::storage::settings::read_settings()
            .ok()
            .and_then(|s| {
                s.thinking_level(&effective_agent.provider, &effective_agent.model)
                    .map(str::to_string)
            });

        let assistant_message =
            assistant_text
                .filter(|text| !text.trim().is_empty())
                .map(|content| ConversationMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: ConversationMessageRole::Assistant,
                    content,
                    created_at: now_rfc3339(),
                    agent: Some(effective_agent),
                    thinking_level,
                });

        if let Some(message) = &assistant_message {
            messages.push(message.clone());
            write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        }

        summary.message_count = messages.len();
        summary.status = status;
        summary.updated_at = now_rfc3339();
        summary.active_score_id = None;
        if provider_session_ref.is_some() {
            summary.last_provider_session_ref = provider_session_ref;
        }
        summary.error = error;
        write_summary_unlocked(&summary)?;

        Ok((summary, assistant_message))
    })
}
