//! Session-level chat message persistence (messages.jsonl).
//!
//! Stores user/assistant messages at the session level for conversation continuity.
//! Each line in messages.jsonl is a JSON-serialised `ChatMessage`.

use std::path::PathBuf;

use crate::models::{ChatMessage, ChatRole};

use super::sessions;

/// Returns the path to the messages.jsonl file for a session.
fn messages_path(session_id: &str) -> Result<PathBuf, String> {
    super::validate_id(session_id)?;
    Ok(sessions::session_dir(session_id)?.join("messages.jsonl"))
}

/// Appends a single chat message to the session's messages.jsonl.
/// Uses append-only writes with explicit flush for durability.
pub fn append_message(session_id: &str, message: &ChatMessage) -> Result<(), String> {
    use std::io::Write;

    let path = messages_path(session_id)?;

    let line = serde_json::to_string(message)
        .map_err(|e| format!("Failed to serialise chat message: {e}"))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open messages file: {e}"))?;

    writeln!(file, "{line}").map_err(|e| format!("Failed to append chat message: {e}"))?;

    file.flush()
        .map_err(|e| format!("Failed to flush messages file: {e}"))?;

    Ok(())
}

/// Reads all chat messages for a session from messages.jsonl.
/// Skips malformed lines gracefully with a warning.
pub fn read_messages(session_id: &str) -> Result<Vec<ChatMessage>, String> {
    let path = messages_path(session_id)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read messages file: {e}"))?;

    let mut messages = Vec::new();
    for (line_num, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<ChatMessage>(line) {
            Ok(msg) => messages.push(msg),
            Err(e) => {
                eprintln!(
                    "Warning: Skipping malformed message at line {} in {}: {e}",
                    line_num + 1,
                    path.display()
                );
            }
        }
    }

    Ok(messages)
}

/// Reads the most recent N chat messages for a session.
/// Useful for building session memory context without loading the full history.
pub fn read_recent_messages(session_id: &str, limit: usize) -> Result<Vec<ChatMessage>, String> {
    let all = read_messages(session_id)?;
    let len = all.len();
    if len <= limit {
        return Ok(all);
    }
    Ok(all.into_iter().skip(len - limit).collect())
}

/// Helper to create a user chat message with the current timestamp.
pub fn user_message(content: &str, run_id: Option<String>) -> ChatMessage {
    ChatMessage {
        role: ChatRole::User,
        content: content.to_string(),
        timestamp: super::now_rfc3339(),
        run_id,
    }
}

/// Helper to create an assistant chat message with the current timestamp.
pub fn assistant_message(content: &str, run_id: Option<String>) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_string(),
        timestamp: super::now_rfc3339(),
        run_id,
    }
}
