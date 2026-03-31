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

/// Precomputed stage index layout for the full pipeline.
#[allow(dead_code)]
pub(super) struct StageIndices {
    pub planner_count: usize,
    pub reviewer_count: usize,
    pub plan_merge: usize,
    pub coder: usize,
    pub reviewer_start: usize,
    pub review_merge: usize,
    pub code_fixer: usize,
    pub total: usize,
}

impl StageIndices {
    pub fn new(planner_count: usize, reviewer_count: usize) -> Self {
        Self {
            planner_count,
            reviewer_count,
            plan_merge: planner_count,
            coder: planner_count + 1,
            reviewer_start: planner_count + 2,
            review_merge: planner_count + 2 + reviewer_count,
            code_fixer: planner_count + 3 + reviewer_count,
            total: planner_count + 4 + reviewer_count,
        }
    }
}

/// Pipeline configuration loaded from settings before runtime state is allocated.
pub(super) struct PipelineConfig {
    pub planners: Vec<PipelineAgent>,
    pub planner_count: usize,
    pub merge_agent: PipelineAgent,
    pub coder: PipelineAgent,
    pub reviewers: Vec<PipelineAgent>,
    pub reviewer_count: usize,
    pub indices: StageIndices,
}

/// Pre-allocated runtime state shared by all pipeline handler spawn blocks.
#[allow(dead_code)]
pub(super) struct PipelineSetup {
    pub abort: Arc<AtomicBool>,
    pub score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    pub stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
    pub planners: Vec<PipelineAgent>,
    pub planner_count: usize,
    pub merge_agent: PipelineAgent,
    pub coder: PipelineAgent,
    pub reviewers: Vec<PipelineAgent>,
    pub reviewer_count: usize,
    pub indices: StageIndices,
}

/// Load pipeline settings without allocating runtime state.
pub(super) fn load_pipeline_config() -> Result<PipelineConfig, String> {
    let settings = crate::storage::settings::read_settings()?;
    let config: CodePipelineSettings = settings
        .code_pipeline
        .ok_or("Code pipeline is not configured. Set it up in Agents settings.")?;
    let CodePipelineSettings { planners, coder, .. } = config;

    let planner_count = planners.len();
    if planner_count == 0 {
        return Err("No planners configured".to_string());
    }
    let merge_agent = planners[0].clone();
    let reviewers = planners.clone();
    let reviewer_count = planner_count;

    let indices = StageIndices::new(planner_count, reviewer_count);

    Ok(PipelineConfig {
        planners,
        planner_count,
        merge_agent,
        coder,
        reviewers,
        reviewer_count,
        indices,
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
        coder,
        reviewers,
        reviewer_count,
        indices,
    } = config;

    let abort = persistence::register_abort_flag(workspace_path, conversation_id)?;
    let score_id_slots = persistence::register_pipeline_score_slots(
        workspace_path, conversation_id, indices.total,
    )?;
    let stage_buffers = persistence::register_pipeline_stage_buffers(
        workspace_path, conversation_id, indices.total,
    )?;

    Ok(PipelineSetup {
        abort,
        score_id_slots,
        stage_buffers,
        planners,
        planner_count,
        merge_agent,
        coder,
        reviewers,
        reviewer_count,
        indices,
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

/// Re-emit completed stage status events so the frontend sees them
/// after a reset (e.g. Resume click or feedback round).
/// Emits all stages with index < `up_to_index`.
pub(super) fn re_emit_completed_stages(
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

/// Ensure a generic stage record exists in pipeline.json.
/// Creates it if absent; resets it to Running if present.
pub(super) fn ensure_stage_record(
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

/// Run the review-merge chain: load first reviewer's session ref, ensure
/// stage record, call run_review_merge.
pub(super) async fn run_review_merge_chain(
    app: AppHandle,
    conv_id: String,
    ws: String,
    abort: Arc<AtomicBool>,
    review_merge_agent: PipelineAgent,
    indices: &StageIndices,
    score_id_slots: &[Arc<std::sync::Mutex<Option<String>>>],
    stage_buffers: &[Arc<std::sync::Mutex<String>>],
) -> Option<Result<PipelineStageRecord, (PipelineStageRecord, String)>> {
    let loaded = persistence::load_pipeline_state(&ws, &conv_id)
        .ok()
        .flatten();
    let session_ref = loaded
        .as_ref()
        .and_then(|s| {
            s.stages
                .iter()
                .find(|st| st.stage_index == indices.reviewer_start)
                .and_then(|st| st.provider_session_ref.clone())
        });

    let ref_val = match session_ref {
        Some(v) => v,
        None => {
            eprintln!("[pipeline] No provider_session_ref from first reviewer; skipping review merge");
            return None;
        }
    };

    let label = format!("{} / {}", review_merge_agent.provider, review_merge_agent.model);
    ensure_stage_record(&ws, &conv_id, indices.review_merge, "Review Merge", &label);

    let slot = score_id_slots.get(indices.review_merge).cloned().unwrap_or_default();
    let buf = stage_buffers.get(indices.review_merge).cloned().unwrap_or_default();

    Some(
        pipeline::run_review_merge(
            app, conv_id, ws, abort, slot, buf,
            indices.review_merge, indices.reviewer_count, ref_val, review_merge_agent,
        )
        .await,
    )
}

/// Run the full coding phase: Coder → Reviewers → Review Merge → Code Fixer.
/// Returns the final status and optional error.
pub(super) async fn run_coding_phase(
    app: AppHandle,
    conv_id: String,
    ws: String,
    setup: &PipelineSetup,
    previous_stages: Option<Vec<PipelineStageRecord>>,
) -> (ConversationStatus, Option<String>) {
    let indices = &setup.indices;

    // Check if the Coder stage is already complete (e.g. resuming from Reviewers).
    let coder_already_done = previous_stages
        .as_ref()
        .and_then(|stages| stages.iter().find(|s| s.stage_name == "Coder"))
        .map(|s| s.status == ConversationStatus::Completed)
        .unwrap_or(false);

    let coder_record = if coder_already_done {
        // Re-emit the completed Coder stage for the frontend and use saved record.
        let loaded = persistence::load_pipeline_state(&ws, &conv_id).ok().flatten();
        loaded
            .and_then(|s| s.stages.into_iter().find(|st| st.stage_name == "Coder"))
            .unwrap_or_else(|| PipelineStageRecord {
                stage_index: indices.coder,
                stage_name: "Coder".to_string(),
                agent_label: format!("{} / {}", setup.coder.provider, setup.coder.model),
                status: ConversationStatus::Completed,
                text: String::new(),
                started_at: None,
                finished_at: None,
                score_id: None,
                provider_session_ref: None,
            })
    } else {
        // --- Coder ---
        let coder_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
        ensure_stage_record(&ws, &conv_id, indices.coder, "Coder", &coder_label);

        let coder_slot = setup.score_id_slots.get(indices.coder).cloned().unwrap_or_default();
        let coder_buf = setup.stage_buffers.get(indices.coder).cloned().unwrap_or_default();

        let coder_result = pipeline::run_coder(
            app.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
            coder_slot, coder_buf, indices.coder, setup.coder.clone(),
        )
        .await;

        if setup.abort.load(Ordering::Acquire) {
            return (ConversationStatus::Stopped, None);
        }

        match coder_result {
            Ok(record) => record,
            Err((_, e)) => return (ConversationStatus::Failed, Some(e)),
        }
    };

    // Collect planner stages for reviewer session resumption.
    let planner_stages: Vec<PipelineStageRecord> = persistence::load_pipeline_state(&ws, &conv_id)
        .ok()
        .flatten()
        .map(|s| s.stages.into_iter().take(indices.planner_count).collect())
        .unwrap_or_default();

    // --- Reviewers ---
    let reviewer_slots: Vec<_> = (0..indices.reviewer_count)
        .map(|i| {
            setup.score_id_slots
                .get(indices.reviewer_start + i)
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    let reviewer_bufs: Vec<_> = (0..indices.reviewer_count)
        .map(|i| {
            setup.stage_buffers
                .get(indices.reviewer_start + i)
                .cloned()
                .unwrap_or_default()
        })
        .collect();

    // Ensure reviewer stage records exist before spawning.
    for (i, reviewer) in setup.reviewers.iter().enumerate() {
        let label = format!("{} / {}", reviewer.provider, reviewer.model);
        ensure_stage_record(
            &ws, &conv_id, indices.reviewer_start + i,
            &format!("Reviewer {}", i + 1), &label,
        );
    }

    let reviewer_result = pipeline::run_pipeline_reviewers(
        app.clone(), conv_id.clone(), ws.clone(),
        setup.reviewers.clone(), setup.abort.clone(),
        reviewer_slots, previous_stages, reviewer_bufs,
        &planner_stages, indices.reviewer_start,
    )
    .await;

    if setup.abort.load(Ordering::Acquire) {
        return (ConversationStatus::Stopped, None);
    }

    if let Err(e) = reviewer_result {
        return (ConversationStatus::Failed, Some(e));
    }

    // --- Review Merge ---
    let Some(review_merge_agent) = setup.reviewers.first().cloned() else {
        return (
            ConversationStatus::Failed,
            Some("No reviewer available for Review Merge".to_string()),
        );
    };

    let review_merge_result = run_review_merge_chain(
        app.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
        review_merge_agent, indices,
        &setup.score_id_slots, &setup.stage_buffers,
    )
    .await;

    if setup.abort.load(Ordering::Acquire) {
        return (ConversationStatus::Stopped, None);
    }

    match &review_merge_result {
        Some(Err((_, e))) => return (ConversationStatus::Failed, Some(e.clone())),
        None => {
            return (
                ConversationStatus::Failed,
                Some("Review merge was skipped — no reviewer session ref".to_string()),
            );
        }
        Some(Ok(_)) => {}
    }

    // --- Code Fixer ---
    let coder_session_ref = coder_record
        .provider_session_ref
        .clone()
        .unwrap_or_default();

    if coder_session_ref.is_empty() {
        return (
            ConversationStatus::Failed,
            Some("No coder session ref available for Code Fixer".to_string()),
        );
    }

    let fixer_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
    ensure_stage_record(&ws, &conv_id, indices.code_fixer, "Code Fixer", &fixer_label);

    let fixer_slot = setup.score_id_slots.get(indices.code_fixer).cloned().unwrap_or_default();
    let fixer_buf = setup.stage_buffers.get(indices.code_fixer).cloned().unwrap_or_default();

    let fixer_result = pipeline::run_code_fixer(
        app, conv_id, ws, setup.abort.clone(),
        fixer_slot, fixer_buf, indices.code_fixer, coder_session_ref, setup.coder.clone(),
    )
    .await;

    if setup.abort.load(Ordering::Acquire) {
        return (ConversationStatus::Stopped, None);
    }

    match fixer_result {
        Ok(_) => (ConversationStatus::Completed, None),
        Err((_, e)) => (ConversationStatus::Failed, Some(e)),
    }
}
