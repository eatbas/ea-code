use crate::models::{ConversationMessage, ConversationSummary};
use crate::storage::atomic_write;

use super::paths::{conversation_file_path, messages_file_path};

pub(super) fn read_summary_unlocked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    let path = conversation_file_path(workspace_path, conversation_id);
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read conversation {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Failed to parse conversation {}: {error}", path.display()))
}

pub(super) fn write_summary_unlocked(summary: &ConversationSummary) -> Result<(), String> {
    let path = conversation_file_path(&summary.workspace_path, &summary.id);
    let json = serde_json::to_string_pretty(summary).map_err(|error| {
        format!(
            "Failed to serialise conversation {}: {error}",
            path.display()
        )
    })?;
    atomic_write(&path, &json)
}

pub(super) fn read_messages_unlocked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<ConversationMessage>, String> {
    let path = messages_file_path(workspace_path, conversation_id);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read messages {}: {error}", path.display()))?;

    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<ConversationMessage>(line).map_err(|error| {
                format!(
                    "Failed to parse message entry in {}: {error}",
                    path.display()
                )
            })
        })
        .collect()
}

pub(super) fn write_messages_unlocked(
    workspace_path: &str,
    conversation_id: &str,
    messages: &[ConversationMessage],
) -> Result<(), String> {
    let path = messages_file_path(workspace_path, conversation_id);
    let mut contents = String::new();
    for message in messages {
        let line = serde_json::to_string(message).map_err(|error| {
            format!(
                "Failed to serialise message for {}: {error}",
                path.display()
            )
        })?;
        contents.push_str(&line);
        contents.push('\n');
    }
    atomic_write(&path, &contents)
}
