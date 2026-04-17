//! Startup reattach pass: after the Symphony sidecar is healthy, consult it
//! about every conversation that was marked running when the app went down so
//! we apply Symphony's truth to our persisted state instead of synthesising it
//! from `artifact_exists`.
//!
//! The persistent running registry (see `persistence::registries`) is the list
//! of conversations to check. For each one we query `GET /v1/chat/{score_id}`
//! per running stage: terminal snapshots update the stage record and summary
//! in place, still-running snapshots are handed to a background poll task so
//! we eventually converge, and unreachable Symphony leaves the persisted flag
//! in place so the next startup retries.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

use crate::conversations::events::{EVENT_CONVERSATION_STATUS, EVENT_PIPELINE_STAGE_STATUS};
use crate::conversations::persistence;
use crate::conversations::score_client::{
    fetch_score_snapshot, poll_until_terminal, SymphonyScoreSnapshot,
};
use crate::models::{
    ConversationStatus, ConversationStatusEvent, PipelineStageRecord, PipelineStageStatusEvent,
    PipelineState,
};
use crate::storage::{now_rfc3339, projects};

/// Short grace so per-workspace probes are staggered and we don't fan out
/// N simultaneous HTTPs to the freshly booted sidecar.
const WORKSPACE_STAGGER: Duration = Duration::from_millis(50);

pub async fn run_startup_reattach_pass(app: AppHandle) {
    let projects_list = match projects::list_projects(true) {
        Ok(projects) => projects,
        Err(error) => {
            eprintln!("[reattach] Failed to list projects: {error}");
            return;
        }
    };

    for project in projects_list {
        reattach_workspace(&app, &project.path).await;
        sleep(WORKSPACE_STAGGER).await;
    }
}

async fn reattach_workspace(app: &AppHandle, workspace_path: &str) {
    let ids = match persistence::read_persisted_running_conversations(workspace_path) {
        Ok(ids) => ids,
        Err(error) => {
            eprintln!(
                "[reattach] Failed to read persisted running set for {workspace_path}: {error}"
            );
            return;
        }
    };

    if ids.is_empty() {
        return;
    }

    for conversation_id in ids {
        reattach_conversation(app, workspace_path, &conversation_id).await;
    }
}

async fn reattach_conversation(app: &AppHandle, workspace_path: &str, conversation_id: &str) {
    // `get_conversation` runs `reconcile_stale_running_unlocked`, which is a
    // no-op while the persisted flag is still set (see recovery.rs), so the
    // status we see here is exactly what was on disk.
    let current_status = match persistence::get_conversation(workspace_path, conversation_id) {
        Ok(detail) => detail.summary.status,
        Err(error) => {
            eprintln!(
                "[reattach] {conversation_id}: summary read failed ({error}); clearing persisted flag"
            );
            let _ = persistence::forget_persisted_running_conversation(workspace_path, conversation_id);
            return;
        }
    };

    if current_status != ConversationStatus::Running {
        let _ = persistence::forget_persisted_running_conversation(workspace_path, conversation_id);
        return;
    }

    let pipeline_state = persistence::load_pipeline_state(workspace_path, conversation_id)
        .ok()
        .flatten();
    let running_stages = collect_running_stages(pipeline_state.as_ref());

    if running_stages.is_empty() {
        // No score to probe — clearing the persisted flag lets the existing
        // heuristic-based reconcile run on the next summary read.
        let _ = persistence::forget_persisted_running_conversation(workspace_path, conversation_id);
        return;
    }

    let mut any_still_running = false;
    let mut any_unreachable = false;

    for (stage_index, score_id) in running_stages {
        match fetch_score_snapshot(&score_id).await {
            Ok(snapshot) if snapshot.status.is_terminal() => {
                apply_terminal_stage(
                    app,
                    workspace_path,
                    conversation_id,
                    stage_index,
                    &score_id,
                    &snapshot,
                );
            }
            Ok(_) => {
                any_still_running = true;
                spawn_score_watcher(
                    app.clone(),
                    workspace_path.to_string(),
                    conversation_id.to_string(),
                    stage_index,
                    score_id,
                );
            }
            Err(error) => {
                any_unreachable = true;
                eprintln!(
                    "[reattach] {conversation_id} stage {stage_index}: Symphony unreachable: {error}"
                );
            }
        }
    }

    if any_still_running || any_unreachable {
        // Leave the conversation flagged as Running and keep it in the
        // persisted set. A background watcher will finalise it, or the next
        // startup will retry if Symphony was unreachable.
        return;
    }

    finalise_summary_from_stages(app, workspace_path, conversation_id);
    let _ = persistence::forget_persisted_running_conversation(workspace_path, conversation_id);
}

fn collect_running_stages(state: Option<&PipelineState>) -> Vec<(usize, String)> {
    let Some(state) = state else {
        return Vec::new();
    };
    state
        .stages
        .iter()
        .filter(|stage| stage.status == ConversationStatus::Running)
        .filter_map(|stage| {
            stage
                .score_id
                .clone()
                .map(|score| (stage.stage_index, score))
        })
        .collect()
}

fn apply_terminal_stage(
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
    stage_index: usize,
    score_id: &str,
    snapshot: &SymphonyScoreSnapshot,
) {
    let new_status = snapshot.status.as_conversation_status();
    let final_text = snapshot
        .final_text
        .clone()
        .unwrap_or_else(|| snapshot.accumulated_text.clone());

    let Some(updated) = update_stage_record(
        workspace_path,
        conversation_id,
        stage_index,
        |record| {
            record.status = new_status.clone();
            record.score_id = Some(score_id.to_string());
            if snapshot.provider_session_ref.is_some() {
                record
                    .provider_session_ref
                    .clone_from(&snapshot.provider_session_ref);
            }
            record.text = final_text.clone();
            record.finished_at.get_or_insert_with(now_rfc3339);
        },
    ) else {
        return;
    };

    let _ = app.emit(
        EVENT_PIPELINE_STAGE_STATUS,
        PipelineStageStatusEvent {
            conversation_id: conversation_id.to_string(),
            stage_index: updated.stage_index,
            stage_name: updated.stage_name.clone(),
            status: updated.status.clone(),
            agent_label: updated.agent_label.clone(),
            text: if updated.text.is_empty() {
                None
            } else {
                Some(updated.text.clone())
            },
            started_at: updated.started_at.clone(),
            finished_at: updated.finished_at.clone(),
        },
    );
}

fn update_stage_record<F>(
    workspace_path: &str,
    conversation_id: &str,
    stage_index: usize,
    mutate: F,
) -> Option<PipelineStageRecord>
where
    F: FnOnce(&mut PipelineStageRecord),
{
    let mut state = persistence::load_pipeline_state(workspace_path, conversation_id)
        .ok()
        .flatten()?;
    let stage = state
        .stages
        .iter_mut()
        .find(|stage| stage.stage_index == stage_index)?;
    mutate(stage);
    let updated = stage.clone();
    if let Err(error) = persistence::update_pipeline_stage(workspace_path, conversation_id, &updated) {
        eprintln!(
            "[reattach] Failed to persist stage {stage_index} for {conversation_id}: {error}"
        );
        return None;
    }
    Some(updated)
}

fn finalise_summary_from_stages(app: &AppHandle, workspace_path: &str, conversation_id: &str) {
    let Some(state) = persistence::load_pipeline_state(workspace_path, conversation_id)
        .ok()
        .flatten()
    else {
        return;
    };

    let (status, error) = summary_status_from_stages(&state);
    match persistence::set_status(workspace_path, conversation_id, status, error) {
        Ok(summary) => {
            let _ = app.emit(
                EVENT_CONVERSATION_STATUS,
                ConversationStatusEvent {
                    conversation: summary,
                    message: None,
                },
            );
        }
        Err(error) => eprintln!(
            "[reattach] Failed to finalise summary for {conversation_id}: {error}"
        ),
    }
}

fn summary_status_from_stages(state: &PipelineState) -> (ConversationStatus, Option<String>) {
    if state
        .stages
        .iter()
        .any(|stage| stage.status == ConversationStatus::Failed)
    {
        return (
            ConversationStatus::Failed,
            Some("One or more pipeline stages failed while the app was offline.".to_string()),
        );
    }

    if state
        .stages
        .iter()
        .any(|stage| stage.status == ConversationStatus::Stopped)
    {
        return (ConversationStatus::Stopped, None);
    }

    if !state.stages.is_empty()
        && state
            .stages
            .iter()
            .all(|stage| stage.status == ConversationStatus::Completed)
    {
        return (ConversationStatus::Completed, None);
    }

    // Mixed terminal + idle/pending stages — we cannot infer a clean final
    // state, so flag as failed so the user can decide to resume manually.
    (
        ConversationStatus::Failed,
        Some("Pipeline stopped unexpectedly while the app was offline.".to_string()),
    )
}

fn spawn_score_watcher(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    stage_index: usize,
    score_id: String,
) {
    tauri::async_runtime::spawn(async move {
        // Reattach has no user-owned abort handle after restart, so use a
        // never-set flag. `poll_until_terminal` returns naturally once
        // Symphony reports a terminal status.
        let stop_flag = AtomicBool::new(false);
        let result = poll_until_terminal(&score_id, &stop_flag, |_| Ok(())).await;
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                eprintln!(
                    "[reattach] Background poll for score {score_id} failed: {error}"
                );
                return;
            }
        };

        apply_terminal_stage(
            &app,
            &workspace_path,
            &conversation_id,
            stage_index,
            &score_id,
            &snapshot,
        );

        // If any other running stages remain, leave the conversation in the
        // persisted set for the next watcher/startup to handle.
        let still_running = persistence::load_pipeline_state(&workspace_path, &conversation_id)
            .ok()
            .flatten()
            .map(|state| {
                state
                    .stages
                    .iter()
                    .any(|stage| stage.status == ConversationStatus::Running)
            })
            .unwrap_or(false);

        if still_running {
            return;
        }

        finalise_summary_from_stages(&app, &workspace_path, &conversation_id);
        let _ = persistence::forget_persisted_running_conversation(&workspace_path, &conversation_id);
    });
}
