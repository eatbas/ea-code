//! Session memory utilities for cross-agent continuity.

use crate::db::{self, DbPool};

const MEMORY_RUN_LIMIT: i64 = 6;
const PROMPT_CHAR_LIMIT: usize = 220;
const SUMMARY_CHAR_LIMIT: usize = 700;
const MEMORY_CHAR_CAP: usize = 5_000;

/// Builds a compact memory block from recent run summaries in the same
/// session so different agent backends can share continuity.
pub fn build_session_memory_context(
    db_pool: &DbPool,
    session_id: &str,
    exclude_run_id: Option<&str>,
) -> String {
    let runs = match db::run_detail::list_recent_for_session(
        db_pool,
        session_id,
        MEMORY_RUN_LIMIT,
        exclude_run_id,
    ) {
        Ok(items) => items,
        Err(_) => return String::new(),
    };

    if runs.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        "SESSION MEMORY".to_string(),
        "Use this as factual continuity across prior runs in the same session.".to_string(),
    ];

    for (idx, run) in runs.iter().enumerate() {
        lines.push(format!("Run {}:", idx + 1));
        lines.push(format!("- Run ID: {}", run.id));
        lines.push(format!("- Started At: {}", run.started_at));
        lines.push(format!("- Status: {}", run.status));
        if let Some(verdict) = run.final_verdict.as_deref() {
            lines.push(format!("- Verdict: {verdict}"));
        }
        lines.push(format!(
            "- Prompt: {}",
            truncate_chars(run.prompt.trim(), PROMPT_CHAR_LIMIT)
        ));
        if let Some(summary) = run
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
        sections.push(memory.to_string());
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
