//! Session memory utilities for cross-agent continuity.

use crate::models::ChatRole;
use crate::storage::{messages, runs};

const MEMORY_RUN_LIMIT: usize = 6;
const MESSAGE_HISTORY_LIMIT: usize = 10;
const PROMPT_CHAR_LIMIT: usize = 220;
const SUMMARY_CHAR_LIMIT: usize = 700;
const MEMORY_CHAR_CAP: usize = 5_000;

/// Builds a compact memory block from recent run summaries in the same
/// session so different agent backends can share continuity.
pub fn build_session_memory_context(session_id: &str, exclude_run_id: Option<&str>) -> String {
    // Get all runs for this session
    let run_summaries = match runs::list_runs(session_id) {
        Ok(summaries) => summaries,
        Err(_) => return String::new(),
    };

    // Filter out excluded run and take most recent limit
    let recent_runs: Vec<_> = run_summaries
        .into_iter()
        .filter(|r| exclude_run_id.map_or(true, |id| r.id != id))
        .take(MEMORY_RUN_LIMIT)
        .collect();

    // Load recent chat messages for conversational context
    let recent_messages =
        messages::read_recent_messages(session_id, MESSAGE_HISTORY_LIMIT).unwrap_or_default();

    if recent_runs.is_empty() && recent_messages.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        "SESSION MEMORY".to_string(),
        "Use this as factual continuity across prior runs in the same session.".to_string(),
    ];

    // Include conversation history for cross-run context.
    // Use bracketed labels to prevent agents from role-playing conversation turns.
    if !recent_messages.is_empty() {
        lines.push(String::new());
        lines.push(
            "PRIOR CONVERSATION (read-only reference — do NOT continue or reproduce this dialogue)"
                .to_string(),
        );
        for msg in &recent_messages {
            let role_label = match msg.role {
                ChatRole::User => "[user request]",
                ChatRole::Assistant => "[agent reply]",
            };
            lines.push(format!(
                "{} {}",
                role_label,
                truncate_chars(&msg.content, PROMPT_CHAR_LIMIT)
            ));
        }
    }

    for (idx, run) in recent_runs.iter().enumerate() {
        lines.push(format!("Run {}:", idx + 1));
        lines.push(format!("- Run ID: {}", run.id));
        lines.push(format!("- Started At: {}", run.started_at));
        lines.push(format!("- Status: {:?}", run.status));
        if let Some(ref verdict) = run.final_verdict {
            lines.push(format!("- Verdict: {verdict:?}"));
        }
        lines.push(format!(
            "- Prompt: {}",
            truncate_chars(run.prompt.trim(), PROMPT_CHAR_LIMIT)
        ));
        if let Some(ref summary) = run
            .executive_summary
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            lines.push(format!(
                "- Executive Summary: {}",
                truncate_chars(summary, SUMMARY_CHAR_LIMIT)
            ));
        }
        if !run.files_changed.is_empty() {
            lines.push(format!("- Files Changed: {}", run.files_changed.len()));
            for file in run.files_changed.iter().take(5) {
                lines.push(format!("  * {}", file));
            }
            if run.files_changed.len() > 5 {
                lines.push(format!("  * ... and {} more", run.files_changed.len() - 5));
            }
        }
    }

    truncate_chars(&lines.join("\n"), MEMORY_CHAR_CAP)
}

/// Merges workspace and session context into one shared prompt context.
pub fn merge_shared_context(workspace_context: &str, session_memory: &str) -> String {
    let mut sections = Vec::<String>::new();
    let workspace = workspace_context.trim();
    let memory = session_memory.trim();

    if !workspace.is_empty() {
        sections.push(format!("WORKSPACE CONTEXT\n{workspace}"));
    }
    if !memory.is_empty() {
        if memory.starts_with("SESSION MEMORY") {
            sections.push(memory.to_string());
        } else {
            sections.push(format!("SESSION MEMORY\n{memory}"));
        }
    }

    sections.join("\n\n")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let clipped = text.chars().take(max_chars).collect::<String>();
    format!("{clipped}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_chars_under_limit() {
        let text = "Short text";
        assert_eq!(truncate_chars(text, 100), text);
    }

    #[test]
    fn test_truncate_chars_over_limit() {
        let text = "a".repeat(100);
        let result = truncate_chars(&text, 50);
        assert_eq!(result.len(), 53); // 50 chars + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_merge_shared_context() {
        let workspace = "Git repo at /path";
        let memory = "SESSION MEMORY\nRun 1:";
        let merged = merge_shared_context(workspace, memory);
        assert!(merged.contains("WORKSPACE CONTEXT"));
        assert!(merged.contains("SESSION MEMORY"));
    }

    #[test]
    fn test_merge_shared_context_with_empty() {
        assert_eq!(merge_shared_context("test", ""), "WORKSPACE CONTEXT\ntest");
        assert_eq!(merge_shared_context("", "test"), "SESSION MEMORY\ntest");
        assert_eq!(merge_shared_context("", ""), "");
    }
}
