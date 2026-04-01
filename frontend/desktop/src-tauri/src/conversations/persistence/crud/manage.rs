//! Conversation management: delete, rename, archive, unarchive, pin.

use crate::models::{ConversationStatus, ConversationSummary};
use crate::storage::{now_rfc3339, with_conversations_lock};

use super::super::io::{read_summary_unlocked, write_summary_unlocked};
use super::super::paths::conversation_dir;

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
