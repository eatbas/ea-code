//! Event emission helpers for pipeline stage transitions and artefacts.

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::agents::AgentInput;
use crate::events::*;
use crate::models::*;
use crate::storage::runs;

/// Returns the current time as epoch milliseconds (string).
pub fn epoch_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

/// Emits a stage status transition event.
pub fn emit_stage(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
) {
    emit_stage_with_duration(app, run_id, stage, status, iteration, None);
}

/// Emits a stage status transition event with an optional duration.
pub fn emit_stage_with_duration(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
    duration_ms: Option<u64>,
) {
    let _ = app.emit(
        EVENT_PIPELINE_STAGE,
        PipelineStagePayload {
            run_id: run_id.to_string(),
            stage: stage.clone(),
            status: status.clone(),
            iteration,
            duration_ms,
        },
    );
}

/// Emits a pipeline artefact event so the frontend can display stage outputs,
/// and persists the artefact to disk for historical viewing.
pub fn emit_artifact(
    app: &AppHandle,
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    kind: &str,
    content: &str,
    iteration: u32,
) {
    let _ = app.emit(
        EVENT_PIPELINE_ARTIFACT,
        PipelineArtifactPayload {
            run_id: run_id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            iteration,
        },
    );

    // Persist artefact to disk for historical viewing
    if let Err(e) = runs::write_artifact(workspace_path, session_id, run_id, iteration, kind, content) {
        eprintln!("Warning: Failed to persist artefact '{kind}': {e}");
    }
}

/// Persists the full prompt sent to an agent as a `{kind}_input.md` artifact.
///
/// This captures the complete system + user prompt for debugging and
/// reproducibility. Does not emit a frontend event -- input prompts are
/// only persisted to disk.
pub fn emit_prompt_artifact(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    kind: &str,
    input: &AgentInput,
    iteration: u32,
) {
    let mut content = input.prompt.clone();
    if let Some(ref ctx) = input.context {
        content = format!("{ctx}\n\n---\n\n{content}");
    }
    let input_kind = format!("{kind}_input");
    if let Err(e) = runs::write_artifact(workspace_path, session_id, run_id, iteration, &input_kind, &content) {
        eprintln!("Warning: Failed to persist prompt artifact '{input_kind}': {e}");
    }
}
