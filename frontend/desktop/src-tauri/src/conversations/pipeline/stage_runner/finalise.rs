use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::conversations::sse::SseResult;
use crate::models::ConversationStatus;

pub(super) fn determine_final_status(
    abort: &AtomicBool,
    file_to_watch: &str,
    file_ready: &AtomicBool,
    file_required: bool,
    stage_name: &str,
    result: &Result<SseResult, String>,
) -> ConversationStatus {
    let file_exists = file_ready.load(Ordering::Acquire) || Path::new(file_to_watch).exists();

    if abort.load(Ordering::Acquire) {
        return ConversationStatus::Stopped;
    }
    if file_exists {
        return ConversationStatus::Completed;
    }
    if file_required {
        match result {
            Ok(run_result) => {
                eprintln!(
                    "[pipeline] {stage_name}: stage finished with status {:?} but required file was not created at {file_to_watch}",
                    run_result.status
                );
            }
            Err(error) => {
                eprintln!(
                    "[pipeline] {stage_name}: required file was not created at {file_to_watch}; SSE stream ended with error: {error}"
                );
            }
        }
        return ConversationStatus::Failed;
    }
    match result {
        Ok(run_result) if run_result.status == ConversationStatus::Completed => {
            ConversationStatus::Completed
        }
        Ok(_) => {
            eprintln!(
                "[pipeline] {stage_name}: SSE finished but without Completed status; \
                 treating as Completed because file_required=false"
            );
            ConversationStatus::Completed
        }
        Err(error) => {
            eprintln!(
                "[pipeline] {stage_name}: SSE stream error ({error}); \
                 treating as Completed because file_required=false"
            );
            ConversationStatus::Completed
        }
    }
}

pub(super) fn resolve_stage_text(
    file_to_watch: &str,
    output_buffer: &Arc<std::sync::Mutex<String>>,
    file_required: bool,
    final_status: &ConversationStatus,
    stage_name: &str,
) -> Option<String> {
    if Path::new(file_to_watch).exists() {
        return std::fs::read_to_string(file_to_watch).ok();
    }
    if file_required || *final_status != ConversationStatus::Completed {
        return None;
    }

    let accumulated = read_accumulated_output(output_buffer);
    let fallback = if accumulated.trim().is_empty() {
        format!(
            "# {stage_name} - auto-generated summary\n\n\
             The {stage_name} stage completed but did not write a summary file.\n\
             The agent may have performed its work without producing explicit output."
        )
    } else {
        format!(
            "# {stage_name} - auto-generated summary\n\n\
             The {stage_name} stage completed but did not write a summary file. \
             Below is the captured output from the session.\n\n---\n\n{accumulated}"
        )
    };

    if let Some(parent) = Path::new(file_to_watch).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match std::fs::write(file_to_watch, &fallback) {
        Ok(()) => {
            eprintln!("[pipeline] {stage_name}: wrote fallback marker file to {file_to_watch}");
            Some(fallback)
        }
        Err(error) => {
            eprintln!("[pipeline] {stage_name}: failed to write fallback marker file: {error}");
            Some(fallback)
        }
    }
}

pub(super) fn describe_stage_failure(
    stage_name: &str,
    file_to_watch: &str,
    file_required: bool,
    result: &Result<SseResult, String>,
    score_id: Option<&str>,
    provider_session_ref: Option<&str>,
    accumulated_text: &str,
) -> String {
    let mut lines: Vec<String> = vec![format!("# {stage_name} failed"), String::new()];

    if file_required && !Path::new(file_to_watch).exists() {
        lines.push("The stage did not create its required output artefact.".to_string());
        lines.push(format!("Expected file: `{file_to_watch}`"));
        lines.push(String::new());
    }

    match result {
        Ok(run_result) => {
            lines.push(format!("SSE status: `{:?}`", run_result.status));
            if let Some(exit_code) = run_result.exit_code {
                lines.push(format!("Exit code: `{exit_code}`"));
            }
            if let Some(error) = &run_result.error {
                lines.push(format!("SSE error: `{error}`"));
            }
        }
        Err(error) => {
            lines.push("SSE status: `stream_error`".to_string());
            lines.push(format!("SSE error: `{error}`"));
        }
    }

    lines.push(format!(
        "Score ID captured: `{}`",
        score_id.unwrap_or("none")
    ));
    lines.push(format!(
        "Provider session captured: `{}`",
        provider_session_ref.unwrap_or("none")
    ));
    lines.push(format!(
        "Captured live output: `{}`",
        if accumulated_text.trim().is_empty() {
            "none".to_string()
        } else {
            format!("present ({} chars)", accumulated_text.len())
        }
    ));

    if file_required {
        lines.push(String::new());
        lines.push(
            "This usually means the agent completed or stopped without writing the mandatory summary file."
                .to_string(),
        );
    }

    lines.join("\n")
}

pub(super) fn read_accumulated_output(output_buffer: &Arc<std::sync::Mutex<String>>) -> String {
    output_buffer
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}
