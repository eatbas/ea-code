use crate::models::{
    AgentSelection, ConversationDetail, ConversationMessage, ConversationMessageRole,
    ConversationStatus, ConversationSummary,
};
use crate::storage::{now_rfc3339, with_conversations_lock};

use super::io::{
    read_messages_unlocked, read_summary_unlocked, write_messages_unlocked, write_summary_unlocked,
};
use super::paths::{conversation_dir, conversations_dir};
use super::recovery::{
    load_summary_with_recovery_unlocked, normalise_title, reconcile_stale_running_unlocked,
};
use super::ConversationCleanupStats;

fn build_detail_unlocked(summary: ConversationSummary) -> Result<ConversationDetail, String> {
    let messages = read_messages_unlocked(&summary.workspace_path, &summary.id)?;
    Ok(ConversationDetail { summary, messages })
}

pub fn create_conversation(
    workspace_path: &str,
    agent: AgentSelection,
    initial_prompt: Option<&str>,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let now = now_rfc3339();
        let summary = ConversationSummary {
            id: uuid::Uuid::new_v4().to_string(),
            title: initial_prompt
                .map(normalise_title)
                .unwrap_or_else(|| "New conversation".to_string()),
            workspace_path: workspace_path.to_string(),
            agent,
            status: ConversationStatus::Idle,
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
            last_provider_session_ref: None,
            active_job_id: None,
            error: None,
            archived_at: None,
            pinned_at: None,
        };
        write_summary_unlocked(&summary)?;
        write_messages_unlocked(workspace_path, &summary.id, &[])?;
        build_detail_unlocked(summary)
    })
}

pub fn list_conversations(
    workspace_path: &str,
    include_archived: bool,
) -> Result<Vec<ConversationSummary>, String> {
    with_conversations_lock(|| {
        let dir = conversations_dir(workspace_path);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries: Vec<ConversationSummary> = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|error| {
            format!(
                "Failed to read conversations directory {}: {error}",
                dir.display()
            )
        })? {
            let entry =
                entry.map_err(|error| format!("Failed to read conversation entry: {error}"))?;
            if !entry.path().is_dir() {
                continue;
            }

            let conversation_id = match entry.file_name().to_str() {
                Some(value) => value.to_string(),
                None => continue,
            };
            let (mut summary, recovered) =
                match load_summary_with_recovery_unlocked(workspace_path, &conversation_id) {
                    Ok(Some(summary)) => summary,
                    Ok(None) => {
                        eprintln!(
                            "[conversations] Removed orphaned conversation {conversation_id}"
                        );
                        continue;
                    }
                    Err(e) => {
                        eprintln!("[conversations] Skipping {conversation_id}: {e}");
                        continue;
                    }
                };
            if recovered {
                eprintln!("[conversations] Recovered conversation {conversation_id}");
            }
            if let Err(e) = reconcile_stale_running_unlocked(&mut summary) {
                eprintln!("[conversations] Reconcile failed for {conversation_id}: {e}");
            }
            if include_archived || summary.archived_at.is_none() {
                summaries.push(summary);
            }
        }

        summaries.sort_by(|left, right| {
            right
                .pinned_at
                .is_some()
                .cmp(&left.pinned_at.is_some())
                .then_with(|| right.updated_at.cmp(&left.updated_at))
        });
        Ok(summaries)
    })
}

pub fn cleanup_orphaned_conversations(
    workspace_path: &str,
) -> Result<ConversationCleanupStats, String> {
    with_conversations_lock(|| {
        let dir = conversations_dir(workspace_path);
        if !dir.exists() {
            return Ok(ConversationCleanupStats::default());
        }

        let mut stats = ConversationCleanupStats::default();
        for entry in std::fs::read_dir(&dir).map_err(|error| {
            format!(
                "Failed to read conversations directory {}: {error}",
                dir.display()
            )
        })? {
            let entry =
                entry.map_err(|error| format!("Failed to read conversation entry: {error}"))?;
            if !entry.path().is_dir() {
                continue;
            }

            let conversation_id = match entry.file_name().to_str() {
                Some(value) => value.to_string(),
                None => continue,
            };

            match load_summary_with_recovery_unlocked(workspace_path, &conversation_id) {
                Ok(Some((_summary, true))) => {
                    stats.recovered += 1;
                }
                Ok(Some((_summary, false))) => {}
                Ok(None) => {
                    stats.removed += 1;
                }
                Err(error) => {
                    eprintln!("[conversations] Cleanup skipped {conversation_id}: {error}");
                }
            }
        }

        Ok(stats)
    })
}

pub fn get_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        reconcile_stale_running_unlocked(&mut summary)?;
        build_detail_unlocked(summary)
    })
}

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
        };
        messages.push(user_message);
        summary.message_count = messages.len();
        if summary.message_count == 1 {
            summary.title = normalise_title(prompt);
        }
        summary.status = ConversationStatus::Running;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        summary.error = None;

        write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        write_summary_unlocked(&summary)?;

        Ok(ConversationDetail { summary, messages })
    })
}

pub fn set_active_job_id(
    workspace_path: &str,
    conversation_id: &str,
    job_id: Option<String>,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.active_job_id = job_id;
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
        summary.active_job_id = None;
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
) -> Result<(ConversationSummary, Option<ConversationMessage>), String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;

        let assistant_message =
            assistant_text
                .filter(|text| !text.trim().is_empty())
                .map(|content| ConversationMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: ConversationMessageRole::Assistant,
                    content,
                    created_at: now_rfc3339(),
                });

        if let Some(message) = &assistant_message {
            messages.push(message.clone());
            write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        }

        summary.message_count = messages.len();
        summary.status = status;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        if provider_session_ref.is_some() {
            summary.last_provider_session_ref = provider_session_ref;
        }
        summary.error = error;
        write_summary_unlocked(&summary)?;

        Ok((summary, assistant_message))
    })
}

pub fn delete_conversation(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    with_conversations_lock(|| {
        let summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.status == ConversationStatus::Running {
            return Err("Cannot delete a running conversation".to_string());
        }

        let dir = conversation_dir(workspace_path, conversation_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|error| {
                format!("Failed to delete conversation {}: {error}", dir.display())
            })?;
        }
        Ok(())
    })
}

pub fn rename_conversation(
    workspace_path: &str,
    conversation_id: &str,
    title: &str,
) -> Result<ConversationSummary, String> {
    let trimmed = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return Err("Conversation title must not be empty".to_string());
    }

    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.title = trimmed;
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn archive_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.status == ConversationStatus::Running {
            return Err("Cannot archive a running conversation".to_string());
        }

        if summary.archived_at.is_none() {
            summary.archived_at = Some(now_rfc3339());
            summary.updated_at = now_rfc3339();
            write_summary_unlocked(&summary)?;
        }

        Ok(summary)
    })
}

pub fn unarchive_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.archived_at.is_some() {
            summary.archived_at = None;
            summary.updated_at = now_rfc3339();
            write_summary_unlocked(&summary)?;
        }

        Ok(summary)
    })
}

pub fn set_conversation_pinned(
    workspace_path: &str,
    conversation_id: &str,
    pinned: bool,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.pinned_at = if pinned { Some(now_rfc3339()) } else { None };
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}
