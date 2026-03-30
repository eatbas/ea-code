//! Shared helpers for pipeline lifecycle management.
//!
//! Extracts the common patterns from start_pipeline, resume_pipeline,
//! and send_plan_edit_feedback to eliminate triplication.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::models::{
    CodePipelineSettings, ConversationStatus, ConversationStatusEvent, PipelineAgent,
    PipelineStageRecord, PipelineStageStatusEvent,
};
use crate::storage::now_rfc3339;

use super::super::events::{EVENT_CONVERSATION_STATUS, EVENT_PIPELINE_STAGE_STATUS};
use super::super::persistence;
use super::super::pipeline;

/// Pipeline configuration loaded from settings before runtime state is allocated.
pub(super) struct PipelineConfig {
    pub planners: Vec<PipelineAgent>,
    pub planner_count: usize,
    pub merge_agent: PipelineAgent,
}

/// Pre-allocated runtime state shared by all pipeline handler spawn blocks.
pub(super) struct PipelineSetup {
    pub abort: Arc<AtomicBool>,
    pub score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    pub stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
    pub planners: Vec<PipelineAgent>,
    pub planner_count: usize,
    pub merge_agent: PipelineAgent,
}

/// Load pipeline settings without allocating runtime state.
pub(super) fn load_pipeline_config() -> Result<PipelineConfig, String> {
    let settings = crate::storage::settings::read_settings()?;
    let config: CodePipelineSettings = settings
        .code_pipeline
        .ok_or("Code pipeline is not configured. Set it up in Agents settings.")?;

    let planners = config.planners;
    let planner_count = planners.len();
    if planner_count == 0 {
        return Err("No planners configured".to_string());
    }
    let merge_agent = planners[0].clone();

    Ok(PipelineConfig {
        planners,
        planner_count,
        merge_agent,
    })
}

/// Allocate abort/slot/buffer registries for a specific conversation.
pub(super) fn prepare_pipeline_with_config(
    workspace_path: &str,
    conversation_id: &str,
    config: PipelineConfig,
) -> Result<PipelineSetup, String> {
    let PipelineConfig {
        planners,
        planner_count,
        merge_agent,
    } = config;

    let abort = persistence::register_abort_flag(workspace_path, conversation_id)?;
    let score_id_slots = persistence::register_pipeline_score_slots(
        workspace_path, conversation_id, planner_count + 1,
    )?;
    let stage_buffers = persistence::register_pipeline_stage_buffers(
        workspace_path, conversation_id, planner_count + 1,
    )?;

    Ok(PipelineSetup {
        abort,
        score_id_slots,
        stage_buffers,
        planners,
        planner_count,
        merge_agent,
    })
}

/// Load pipeline settings and allocate abort/slot/buffer registries.
/// Shared by resume_pipeline and send_plan_edit_feedback.
pub(super) fn prepare_pipeline(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<PipelineSetup, String> {
    prepare_pipeline_with_config(workspace_path, conversation_id, load_pipeline_config()?)
}

/// Acquire the running-conversation guard and emit Running status.
/// Returns the guard on success, or logs and returns None if tracking failed.
pub(super) fn begin_pipeline_task(
    app: &AppHandle,
    ws: &str,
    conv_id: &str,
) -> Option<persistence::RunningConversationGuard> {
    let guard = match persistence::track_running_conversation(ws, conv_id) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[pipeline] Failed to track running conversation: {e}");
            return None;
        }
    };
    emit_running_status(app, ws, conv_id);
    Some(guard)
}

/// Set conversation status to Running and emit the event.
pub(super) fn emit_running_status(app: &AppHandle, ws: &str, conv_id: &str) {
    match persistence::set_status(ws, conv_id, ConversationStatus::Running, None) {
        Ok(summary) => {
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
pub(super) fn emit_final_status(
    app: &AppHandle,
    ws: &str,
    conv_id: &str,
    status: ConversationStatus,
    error: Option<String>,
) {
    match persistence::set_status(ws, conv_id, status, error) {
        Ok(summary) => {
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
pub(super) fn pipeline_cleanup(ws: &str, conv_id: &str) {
    let _ = persistence::remove_pipeline_stage_buffers(ws, conv_id);
    let _ = persistence::remove_pipeline_score_slots(ws, conv_id);
    let _ = persistence::remove_abort_flag(ws, conv_id);
}

/// Determine the final conversation status from planner + merge results.
pub(super) fn determine_final_status(
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

/// Re-emit completed planner stage status events so the frontend sees them
/// after a reset (e.g. Resume click or feedback round).
pub(super) fn re_emit_completed_stages(
    app: &AppHandle,
    conv_id: &str,
    ws: &str,
    planner_count: usize,
) {
    if let Ok(Some(saved)) = persistence::load_pipeline_state(ws, conv_id) {
        for stage in saved.stages.iter().take(planner_count) {
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
                },
            );
        }
    }
}

/// Ensure the Plan Merge stage record exists in pipeline.json.
/// If it doesn't exist, creates it. If it does, marks it Running.
pub(super) fn ensure_merge_stage_record(
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

/// Run the merge chain: load session ref, ensure record, call run_plan_merge.
/// Returns None if the merge was skipped (no session ref available).
pub(super) async fn run_merge_chain(
    app: AppHandle,
    conv_id: String,
    ws: String,
    abort: Arc<AtomicBool>,
    merge_agent: crate::models::PipelineAgent,
    planner_count: usize,
    score_id_slots: &[Arc<std::sync::Mutex<Option<String>>>],
    stage_buffers: &[Arc<std::sync::Mutex<String>>],
) -> Option<Result<PipelineStageRecord, (PipelineStageRecord, String)>> {
    let loaded = persistence::load_pipeline_state(&ws, &conv_id)
        .ok()
        .flatten();
    let session_ref = loaded
        .as_ref()
        .and_then(|s| s.stages.first().and_then(|st| st.provider_session_ref.clone()));

    let ref_val = match session_ref {
        Some(v) => v,
        None => {
            eprintln!("[pipeline] No provider_session_ref from first planner; skipping merge");
            return None;
        }
    };

    let merge_label = format!("{} / {}", merge_agent.provider, merge_agent.model);
    ensure_merge_stage_record(&ws, &conv_id, planner_count, &merge_label);

    let merge_slot = score_id_slots.get(planner_count).cloned().unwrap_or_default();
    let merge_buf = stage_buffers.get(planner_count).cloned().unwrap_or_default();

    Some(
        pipeline::run_plan_merge(
            app, conv_id, ws, abort, merge_slot, merge_buf,
            planner_count, ref_val, merge_agent,
        )
        .await,
    )
}
