use tauri::{AppHandle, Emitter};

use crate::conversations::events::EVENT_PIPELINE_DEBUG_LOG;
use crate::conversations::persistence;
use crate::models::PipelineDebugLogEvent;
use crate::storage::now_rfc3339;

pub fn emit_pipeline_debug(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    message: impl Into<String>,
) {
    let created_at = now_rfc3339();
    let message = message.into();
    let line = format!("[{created_at}] {message}");

    if let Err(error) =
        persistence::append_pipeline_debug_log(workspace_path, conversation_id, &line)
    {
        eprintln!("[pipeline-debug] Failed to persist log line: {error}");
    }

    if let Err(error) = app.emit(
        EVENT_PIPELINE_DEBUG_LOG,
        PipelineDebugLogEvent {
            conversation_id: conversation_id.to_string(),
            created_at,
            line: line.clone(),
        },
    ) {
        eprintln!("[pipeline-debug] Failed to emit debug log event: {error}");
    }

    eprintln!("[pipeline-debug] {conversation_id}: {message}");
}
