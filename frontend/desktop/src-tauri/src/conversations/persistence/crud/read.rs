//! Conversation read operations: list, get, create, and cleanup.

use crate::models::{
    AgentSelection, ConversationDetail, ConversationStatus, ConversationSummary,
};
use crate::storage::{now_rfc3339, with_conversations_lock};

use super::super::io::{
    read_messages_unlocked, read_summary_unlocked, write_messages_unlocked, write_summary_unlocked,
};
use super::super::paths::conversations_dir;
use super::super::recovery::{
    load_summary_with_recovery_unlocked, normalise_title, reconcile_stale_running_unlocked,
};
use super::super::ConversationCleanupStats;

pub(in crate::conversations::persistence) fn build_detail_unlocked(
    summary: ConversationSummary,
) -> Result<ConversationDetail, String> {
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
            active_score_id: None,
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
