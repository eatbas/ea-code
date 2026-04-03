//! Pipeline lifecycle helpers: status emission, cleanup, and guards.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::models::{
    ConversationStatus, ConversationStatusEvent, PipelineStageRecord, PipelineStageStatusEvent,
};
use crate::storage::now_rfc3339;

use super::super::super::pipeline_debug::emit_pipeline_debug;
use super::super::super::events::{EVENT_CONVERSATION_STATUS, EVENT_PIPELINE_STAGE_STATUS};
use super::super::super::persistence;

/// Acquire the running-conversation guard and emit Running status.
/// Returns the guard on success, or logs and returns None if tracking failed.
pub(in crate::conversations::commands) fn begin_pipeline_task(
    app: &AppHandle,
    ws: &str,
    conv_id: &str,
) -> Option<persistence::RunningConversationGuard> {
    let guard = match persistence::track_running_conversation(ws, conv_id) {
        Ok(g) => g,
        Err(e) => {
            emit_pipeline_debug(app, ws, conv_id, format!("Pipeline start skipped: {e}"));
            eprintln!("[pipeline] Failed to track running conversation: {e}");
            return None;
        }
    };
    emit_pipeline_debug(app, ws, conv_id, "Pipeline task acquired running guard");
    emit_running_status(app, ws, conv_id);
    Some(guard)
}

/// Set conversation status to Running and emit the event.
pub(in crate::conversations::commands) fn emit_running_status(
    app: &AppHandle,
    ws: &str,
    conv_id: &str,
) {
    match persistence::set_status(ws, conv_id, ConversationStatus::Running, None) {
        Ok(summary) => {
            emit_pipeline_debug(app, ws, conv_id, "Conversation status -> running");
            let _ = app.emit(
                EVENT_CONVERSATION_STATUS,
                ConversationStatusEvent {
                    conversation: summary,
                    message: None,
                },
            );
        }
        Err(e) => eprintln!("[pipeline] Failed to set running status: {e}"),
    }
}

/// Set final conversation status and emit the event.
pub(in crate::conversations::commands) fn emit_final_status(
    app: &AppHandle,
    ws: &str,
    conv_id: &str,
    status: ConversationStatus,
    error: Option<String>,
) {
    let status_label = format!("{:?}", status).to_lowercase();
    let debug_message = match &error {
        Some(error) => format!("Conversation final status -> {status_label}; error={error}"),
        None => format!("Conversation final status -> {status_label}"),
    };
    match persistence::set_status(ws, conv_id, status, error) {
        Ok(summary) => {
            emit_pipeline_debug(app, ws, conv_id, debug_message);
            let _ = app.emit(
                EVENT_CONVERSATION_STATUS,
                ConversationStatusEvent {
                    conversation: summary,
                    message: None,
                },
            );
        }
        Err(e) => eprintln!("[pipeline] Failed to set final status: {e}"),
    }
}

/// Remove all pipeline runtime registries for a finished conversation.
pub(in crate::conversations::commands) fn pipeline_cleanup(ws: &str, conv_id: &str) {
    let _ = persistence::remove_pipeline_stage_buffers(ws, conv_id);
    let _ = persistence::remove_pipeline_score_slots(ws, conv_id);
    let _ = persistence::remove_abort_flag(ws, conv_id);
}

/// Determine the final conversation status from planner + merge results.
pub(in crate::conversations::commands) fn determine_final_status(
    abort: &Arc<AtomicBool>,
    planner_result: &Result<(), String>,
    merge_result: &Option<Result<PipelineStageRecord, (PipelineStageRecord, String)>>,
) -> (ConversationStatus, Option<String>) {
    let status = if abort.load(Ordering::Acquire) {
        ConversationStatus::Stopped
    } else if planner_result.is_err() {
        ConversationStatus::Failed
    } else {
        match merge_result {
            Some(Ok(_)) => ConversationStatus::AwaitingReview,
            Some(Err(_)) => ConversationStatus::Failed,
            None if planner_result.is_ok() => ConversationStatus::AwaitingReview,
            None => ConversationStatus::Failed,
        }
    };

    let error = planner_result
        .as_ref()
        .err()
        .cloned()
        .or_else(|| {
            merge_result
                .as_ref()
                .and_then(|r| r.as_ref().err().map(|(_, e)| e.clone()))
        });

    (status, error)
}

/// Re-emit completed stage status events so the frontend sees them
/// after a reset (e.g. Resume click or feedback round).
/// Emits all stages with index < `up_to_index`.
pub(in crate::conversations::commands) fn re_emit_completed_stages(
    app: &AppHandle,
    conv_id: &str,
    ws: &str,
    up_to_index: usize,
) {
    if let Ok(Some(saved)) = persistence::load_pipeline_state(ws, conv_id) {
        for stage in saved.stages.iter().filter(|s| s.stage_index < up_to_index) {
            let _ = app.emit(
                EVENT_PIPELINE_STAGE_STATUS,
                PipelineStageStatusEvent {
                    conversation_id: conv_id.to_string(),
                    stage_index: stage.stage_index,
                    stage_name: stage.stage_name.clone(),
                    status: stage.status.clone(),
                    agent_label: stage.agent_label.clone(),
                    text: if stage.text.is_empty() {
                        None
                    } else {
                        Some(stage.text.clone())
                    },
                    started_at: stage.started_at.clone(),
                    finished_at: stage.finished_at.clone(),
                },
            );
        }
    }
}

/// Ensure the Plan Merge stage record exists in pipeline.json.
/// If it doesn't exist, creates it. If it does, marks it Running.
pub(in crate::conversations::commands) fn ensure_merge_stage_record(
    ws: &str,
    conv_id: &str,
    planner_count: usize,
    merge_label: &str,
) {
    if let Ok(Some(mut state)) = persistence::load_pipeline_state(ws, conv_id) {
        if !state.stages.iter().any(|s| s.stage_name == "Plan Merge") {
            state.stages.push(PipelineStageRecord {
                stage_index: planner_count,
                stage_name: "Plan Merge".to_string(),
                agent_label: merge_label.to_string(),
                status: ConversationStatus::Running,
                text: String::new(),
                started_at: Some(now_rfc3339()),
                finished_at: None,
                score_id: None,
                provider_session_ref: None,
            });
        } else if let Some(merge) = state
            .stages
            .iter_mut()
            .find(|s| s.stage_name == "Plan Merge")
        {
            merge.status = ConversationStatus::Running;
            merge.started_at = Some(now_rfc3339());
            merge.finished_at = None;
        }
        let _ = persistence::save_pipeline_state(ws, conv_id, &state);
    }
}

/// Ensure a generic stage record exists in pipeline.json.
/// Creates it if absent; resets it to Running if present.
pub(in crate::conversations::commands) fn ensure_stage_record(
    ws: &str,
    conv_id: &str,
    stage_index: usize,
    stage_name: &str,
    agent_label: &str,
) {
    if let Ok(Some(mut state)) = persistence::load_pipeline_state(ws, conv_id) {
        if let Some(existing) = state.stages.iter_mut().find(|s| s.stage_index == stage_index) {
            existing.status = ConversationStatus::Running;
            existing.started_at = Some(now_rfc3339());
            existing.finished_at = None;
        } else {
            state.stages.push(PipelineStageRecord {
                stage_index,
                stage_name: stage_name.to_string(),
                agent_label: agent_label.to_string(),
                status: ConversationStatus::Running,
                text: String::new(),
                started_at: Some(now_rfc3339()),
                finished_at: None,
                score_id: None,
                provider_session_ref: None,
            });
            state.stages.sort_by_key(|s| s.stage_index);
        }
        let _ = persistence::save_pipeline_state(ws, conv_id, &state);
    }
}
