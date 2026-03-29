use serde::Deserialize;
use tauri::AppHandle;
use tokio::time::{sleep, Duration, Instant};

use crate::commands::api_health::hive_api_base_url;
use tauri::Emitter;

use crate::models::{
    AgentSelection, ConversationDetail, ConversationStatus, ConversationStatusEvent,
    ConversationSummary, PipelineStageRecord, PipelineStageStatusEvent, PipelineState,
};
use crate::storage::now_rfc3339;

use super::chat;
use super::events::{EVENT_CONVERSATION_STATUS, EVENT_PIPELINE_STAGE_STATUS};
use super::persistence;
use super::pipeline;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    status: ConversationStatus,
}

const STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const PIPELINE_STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(3);

async fn send_hive_stop_request(
    client: &reqwest::Client,
    job_id: &str,
) -> Result<(), String> {
    let url = format!("{}/v1/chat/{job_id}/stop", hive_api_base_url());
    let response = client
        .post(url)
        .send()
        .await
        .map_err(|error| format!("Failed to stop hive job {job_id}: {error}"))?;
    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!(
        "Failed to stop hive job {job_id}: HTTP {status} — {body}"
    ))
}

async fn wait_for_stoppable_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    let deadline = Instant::now() + STOP_WAIT_TIMEOUT;

    loop {
        let summary = persistence::get_conversation(workspace_path, conversation_id)?.summary;
        if summary.active_job_id.is_some() || summary.status != ConversationStatus::Running {
            return Ok(summary);
        }
        if Instant::now() >= deadline {
            return Ok(summary);
        }

        sleep(STOP_WAIT_POLL_INTERVAL).await;
    }
}

#[tauri::command]
pub async fn list_workspace_conversations(
    workspace_path: String,
    include_archived: Option<bool>,
) -> Result<Vec<ConversationSummary>, String> {
    persistence::list_conversations(&workspace_path, include_archived.unwrap_or(false))
}

#[tauri::command]
pub async fn create_conversation(
    workspace_path: String,
    agent: AgentSelection,
    initial_prompt: Option<String>,
) -> Result<ConversationDetail, String> {
    persistence::create_conversation(&workspace_path, agent, initial_prompt.as_deref())
}

#[tauri::command]
pub async fn get_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationDetail, String> {
    persistence::get_conversation(&workspace_path, &conversation_id)
}

#[tauri::command]
pub async fn send_conversation_turn(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    prompt: String,
) -> Result<ConversationDetail, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Prompt must not be empty".to_string());
    }

    let detail = persistence::mark_turn_running(&workspace_path, &conversation_id, trimmed)?;
    let abort = persistence::register_abort_flag(&workspace_path, &conversation_id)?;
    let app_handle = app.clone();
    let detail_for_task = detail.clone();
    let prompt_for_task = trimmed.to_string();
    let tracked_workspace_path = detail.summary.workspace_path.clone();
    let tracked_conversation_id = detail.summary.id.clone();

    tokio::spawn(async move {
        let _running_guard = match persistence::track_running_conversation(
            &tracked_workspace_path,
            &tracked_conversation_id,
        ) {
            Ok(guard) => guard,
            Err(error) => {
                eprintln!("[conversation] Failed to track running conversation: {error}");
                return;
            }
        };

        if let Err(error) =
            chat::run_conversation_turn(app_handle, detail_for_task, prompt_for_task, abort).await
        {
            eprintln!("[conversation] Failed to run conversation turn: {error}");
        }
        let _ = persistence::remove_abort_flag(&tracked_workspace_path, &tracked_conversation_id);
    });

    Ok(detail)
}

#[tauri::command]
pub async fn stop_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    let mut summary = persistence::get_conversation(&workspace_path, &conversation_id)?.summary;
    if summary.status == ConversationStatus::Running && summary.active_job_id.is_none() {
        summary = wait_for_stoppable_conversation(&workspace_path, &conversation_id).await?;
    }

    let Some(job_id) = summary.active_job_id.clone() else {
        if summary.status == ConversationStatus::Running {
            return Err("The run is still starting and cannot be stopped yet. Please try again in a moment.".to_string());
        }
        return Ok(summary);
    };

    persistence::trigger_abort(&workspace_path, &conversation_id)?;

    let url = format!("{}/v1/chat/{job_id}/stop", hive_api_base_url());
    let response = reqwest::Client::new()
        .post(url)
        .send()
        .await
        .map_err(|error| format!("Failed to stop conversation: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Failed to stop conversation: HTTP {status} — {body}"
        ));
    }

    let _stop_response = response
        .json::<StopResponse>()
        .await
        .map_err(|error| format!("Failed to parse stop response: {error}"))?;

    if _stop_response.status == ConversationStatus::Stopped {
        return persistence::set_status(
            &workspace_path,
            &conversation_id,
            ConversationStatus::Stopped,
            None,
        );
    }

    persistence::get_conversation(&workspace_path, &conversation_id).map(|detail| detail.summary)
}

#[tauri::command]
pub async fn delete_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<(), String> {
    persistence::delete_conversation(&workspace_path, &conversation_id)
}

#[tauri::command]
pub async fn rename_conversation(
    workspace_path: String,
    conversation_id: String,
    title: String,
) -> Result<ConversationSummary, String> {
    persistence::rename_conversation(&workspace_path, &conversation_id, &title)
}

#[tauri::command]
pub async fn archive_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    persistence::archive_conversation(&workspace_path, &conversation_id)
}

#[tauri::command]
pub async fn unarchive_conversation(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    persistence::unarchive_conversation(&workspace_path, &conversation_id)
}

#[tauri::command]
pub async fn set_conversation_pinned(
    workspace_path: String,
    conversation_id: String,
    pinned: bool,
) -> Result<ConversationSummary, String> {
    persistence::set_conversation_pinned(&workspace_path, &conversation_id, pinned)
}

#[tauri::command]
pub async fn start_pipeline(
    app: AppHandle,
    workspace_path: String,
    prompt: String,
) -> Result<ConversationDetail, String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("Prompt must not be empty".to_string());
    }

    let settings = crate::storage::settings::read_settings()?;
    let pipeline_config = settings
        .code_pipeline
        .ok_or("Code pipeline is not configured. Set it up in Agents settings.".to_string())?;

    // Use the first planner as the conversation agent for display purposes.
    let agent = pipeline_config
        .planners
        .first()
        .map(|p| AgentSelection {
            provider: p.provider.clone(),
            model: p.model.clone(),
        })
        .ok_or("No planners configured".to_string())?;

    let detail = persistence::create_conversation(&workspace_path, agent, Some(trimmed))?;
    let conversation_id = detail.summary.id.clone();
    let abort = persistence::register_abort_flag(&workspace_path, &conversation_id)?;

    let planners = pipeline_config.planners;
    let planner_count = planners.len();
    // Allocate planner_count + 1 slots: one extra for the Plan Merge stage.
    let job_id_slots =
        persistence::register_pipeline_job_slots(&workspace_path, &conversation_id, planner_count + 1)?;
    let stage_buffers =
        persistence::register_pipeline_stage_buffers(&workspace_path, &conversation_id, planner_count + 1)?;

    // Keep the first planner's config for the merge phase.
    let merge_agent = planners[0].clone();

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let user_prompt = trimmed.to_string();

    tokio::spawn(async move {
        let _guard = match persistence::track_running_conversation(&ws, &conv_id) {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("[pipeline] Failed to track running conversation: {e}");
                return;
            }
        };

        match persistence::set_status(&ws, &conv_id, ConversationStatus::Running, None) {
            Ok(summary) => {
                let _ = app_handle.emit(
                    EVENT_CONVERSATION_STATUS,
                    ConversationStatusEvent {
                        conversation: summary,
                        message: None,
                    },
                );
            }
            Err(e) => eprintln!("[pipeline] Failed to set running status: {e}"),
        }

        let planner_result = pipeline::run_pipeline_planners(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            planners,
            user_prompt,
            abort.clone(),
            job_id_slots[..planner_count].to_vec(),
            None,
            stage_buffers[..planner_count].to_vec(),
        )
        .await;

        // Chain the plan-merge phase if planners succeeded.
        let merge_result = if planner_result.is_ok()
            && !abort.load(std::sync::atomic::Ordering::Acquire)
        {
            // Get the first planner's provider_session_ref from saved state.
            let loaded = persistence::load_pipeline_state(&ws, &conv_id)
                .ok()
                .flatten();
            let session_ref = loaded
                .as_ref()
                .and_then(|s| s.stages.first().and_then(|st| st.provider_session_ref.clone()));

            if let Some(ref_val) = session_ref {
                // Add the Plan Merge stage record to pipeline.json so that
                // update_pipeline_stage can find it at the correct index.
                if let Some(mut state) = loaded {
                    let merge_label = format!(
                        "{} / {}",
                        merge_agent.provider, merge_agent.model
                    );
                    state.stages.push(PipelineStageRecord {
                        stage_index: planner_count,
                        stage_name: "Plan Merge".to_string(),
                        agent_label: merge_label,
                        status: ConversationStatus::Running,
                        text: String::new(),
                        started_at: Some(now_rfc3339()),
                        finished_at: None,
                        job_id: None,
                        provider_session_ref: None,
                    });
                    let _ = persistence::save_pipeline_state(&ws, &conv_id, &state);
                }

                let merge_slot = job_id_slots.get(planner_count).cloned().unwrap_or_default();
                let merge_buf = stage_buffers.get(planner_count).cloned().unwrap_or_default();
                Some(
                    pipeline::run_plan_merge(
                        app_handle.clone(),
                        conv_id.clone(),
                        ws.clone(),
                        abort.clone(),
                        merge_slot,
                        merge_buf,
                        planner_count,
                        ref_val,
                        merge_agent,
                    )
                    .await,
                )
            } else {
                eprintln!("[pipeline] No provider_session_ref from first planner; skipping merge");
                None
            }
        } else {
            None
        };

        let final_status = if abort.load(std::sync::atomic::Ordering::Acquire) {
            ConversationStatus::Stopped
        } else if planner_result.is_err() {
            ConversationStatus::Failed
        } else {
            match &merge_result {
                Some(Ok(_)) => ConversationStatus::Completed,
                Some(Err(_)) => ConversationStatus::Failed,
                None if planner_result.is_ok() => ConversationStatus::Completed,
                None => ConversationStatus::Failed,
            }
        };
        let error = planner_result.err().or_else(|| {
            merge_result.and_then(|r| r.err().map(|(_, e)| e))
        });

        match persistence::set_status(&ws, &conv_id, final_status, error) {
            Ok(summary) => {
                let _ = app_handle.emit(
                    EVENT_CONVERSATION_STATUS,
                    ConversationStatusEvent {
                        conversation: summary,
                        message: None,
                    },
                );
            }
            Err(e) => eprintln!("[pipeline] Failed to set final status: {e}"),
        }

        let _ = persistence::remove_pipeline_stage_buffers(&ws, &conv_id);
        let _ = persistence::remove_pipeline_job_slots(&ws, &conv_id);
        let _ = persistence::remove_abort_flag(&ws, &conv_id);
    });

    Ok(detail)
}

#[tauri::command]
pub async fn stop_pipeline(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    // Set the local abort flag so SSE consumers exit immediately.
    persistence::trigger_abort(&workspace_path, &conversation_id)?;

    // Stop every job we can see now, then poll briefly for late-arriving
    // run_started events so planners that publish a job ID just after the
    // user presses Stop are also cancelled.
    let deadline = Instant::now() + PIPELINE_STOP_WAIT_TIMEOUT;
    let client = reqwest::Client::new();
    let mut stopped_job_ids = std::collections::HashSet::new();

    loop {
        let job_ids = persistence::get_pipeline_job_ids(&workspace_path, &conversation_id)?;
        for job_id in job_ids {
            if stopped_job_ids.insert(job_id.clone()) {
                if let Err(e) = send_hive_stop_request(&client, &job_id).await {
                    eprintln!("[pipeline] {e}");
                }
            }
        }

        if Instant::now() >= deadline {
            break;
        }

        sleep(STOP_WAIT_POLL_INTERVAL).await;
    }

    persistence::mark_running_pipeline_stages_stopped(&workspace_path, &conversation_id)?;

    persistence::set_status(
        &workspace_path,
        &conversation_id,
        ConversationStatus::Stopped,
        None,
    )
}

#[tauri::command]
pub async fn resume_pipeline(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationDetail, String> {
    let state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    let settings = crate::storage::settings::read_settings()?;
    let pipeline_config = settings
        .code_pipeline
        .ok_or("Code pipeline is not configured. Set it up in Agents settings.".to_string())?;

    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    let abort = persistence::register_abort_flag(&workspace_path, &conversation_id)?;

    let user_prompt = state.user_prompt;
    let previous_stages = state.stages;
    let planners = pipeline_config.planners;
    let planner_count = planners.len();
    let merge_agent = planners[0].clone();

    // Allocate planner_count + 1 slots: one extra for the Plan Merge stage.
    let job_id_slots =
        persistence::register_pipeline_job_slots(&workspace_path, &conversation_id, planner_count + 1)?;
    let stage_buffers =
        persistence::register_pipeline_stage_buffers(&workspace_path, &conversation_id, planner_count + 1)?;

    // Determine whether all planner stages are already completed and only the
    // merge stage needs (re-)running.
    let all_planners_done = previous_stages.iter()
        .take(planner_count)
        .all(|s| s.status == ConversationStatus::Completed);
    let merge_needs_run = previous_stages.get(planner_count)
        .map(|s| s.status != ConversationStatus::Completed)
        .unwrap_or(true); // No merge stage yet means it needs to run.

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();

    tokio::spawn(async move {
        let _guard = match persistence::track_running_conversation(&ws, &conv_id) {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("[pipeline] Failed to track running conversation: {e}");
                return;
            }
        };

        match persistence::set_status(&ws, &conv_id, ConversationStatus::Running, None) {
            Ok(summary) => {
                let _ = app_handle.emit(
                    EVENT_CONVERSATION_STATUS,
                    ConversationStatusEvent {
                        conversation: summary,
                        message: None,
                    },
                );
            }
            Err(e) => eprintln!("[pipeline] Failed to set running status: {e}"),
        }

        // Run planners if any still need to complete.
        let planner_result = if all_planners_done {
            // Planners are already done — re-emit their status and text so
            // the frontend (which was reset) sees them as completed.
            if let Ok(Some(saved)) = persistence::load_pipeline_state(&ws, &conv_id) {
                for stage in saved.stages.iter().take(planner_count) {
                    let _ = app_handle.emit(
                        EVENT_PIPELINE_STAGE_STATUS,
                        PipelineStageStatusEvent {
                            conversation_id: conv_id.clone(),
                            stage_index: stage.stage_index,
                            stage_name: stage.stage_name.clone(),
                            status: stage.status.clone(),
                            agent_label: stage.agent_label.clone(),
                            text: if stage.text.is_empty() { None } else { Some(stage.text.clone()) },
                        },
                    );
                }
            }
            Ok(())
        } else {
            pipeline::run_pipeline_planners(
                app_handle.clone(),
                conv_id.clone(),
                ws.clone(),
                planners,
                user_prompt,
                abort.clone(),
                job_id_slots[..planner_count].to_vec(),
                Some(previous_stages),
                stage_buffers[..planner_count].to_vec(),
            )
            .await
        };

        // Chain the plan-merge phase if planners succeeded and merge needs running.
        let merge_result = if planner_result.is_ok()
            && merge_needs_run
            && !abort.load(std::sync::atomic::Ordering::Acquire)
        {
            let loaded = persistence::load_pipeline_state(&ws, &conv_id)
                .ok()
                .flatten();
            let session_ref = loaded
                .as_ref()
                .and_then(|s| s.stages.first().and_then(|st| st.provider_session_ref.clone()));

            if let Some(ref_val) = session_ref {
                // Ensure the Plan Merge stage record exists in pipeline.json
                // so that update_pipeline_stage can update it at the correct index.
                if let Some(mut state) = loaded {
                    // Only add the record if it doesn't already exist (e.g. retrying
                    // a failed merge should not duplicate it).
                    if !state.stages.iter().any(|s| s.stage_name == "Plan Merge") {
                        let merge_label = format!(
                            "{} / {}",
                            merge_agent.provider, merge_agent.model
                        );
                        state.stages.push(PipelineStageRecord {
                            stage_index: planner_count,
                            stage_name: "Plan Merge".to_string(),
                            agent_label: merge_label,
                            status: ConversationStatus::Running,
                            text: String::new(),
                            started_at: Some(now_rfc3339()),
                            finished_at: None,
                            job_id: None,
                            provider_session_ref: None,
                        });
                    } else {
                        // Mark the existing merge stage as running again.
                        if let Some(merge) = state.stages.iter_mut().find(|s| s.stage_name == "Plan Merge") {
                            merge.status = ConversationStatus::Running;
                            merge.started_at = Some(now_rfc3339());
                            merge.finished_at = None;
                        }
                    }
                    let _ = persistence::save_pipeline_state(&ws, &conv_id, &state);
                }

                let merge_slot = job_id_slots.get(planner_count).cloned().unwrap_or_default();
                let merge_buf = stage_buffers.get(planner_count).cloned().unwrap_or_default();
                Some(
                    pipeline::run_plan_merge(
                        app_handle.clone(),
                        conv_id.clone(),
                        ws.clone(),
                        abort.clone(),
                        merge_slot,
                        merge_buf,
                        planner_count,
                        ref_val,
                        merge_agent,
                    )
                    .await,
                )
            } else {
                eprintln!("[pipeline] No provider_session_ref from first planner; skipping merge");
                None
            }
        } else {
            None
        };

        let final_status = if abort.load(std::sync::atomic::Ordering::Acquire) {
            ConversationStatus::Stopped
        } else if planner_result.is_err() {
            ConversationStatus::Failed
        } else {
            match &merge_result {
                Some(Ok(_)) => ConversationStatus::Completed,
                Some(Err(_)) => ConversationStatus::Failed,
                None if planner_result.is_ok() => ConversationStatus::Completed,
                None => ConversationStatus::Failed,
            }
        };
        let error = planner_result.err().or_else(|| {
            merge_result.and_then(|r| r.err().map(|(_, e)| e))
        });

        match persistence::set_status(&ws, &conv_id, final_status, error) {
            Ok(summary) => {
                let _ = app_handle.emit(
                    EVENT_CONVERSATION_STATUS,
                    ConversationStatusEvent {
                        conversation: summary,
                        message: None,
                    },
                );
            }
            Err(e) => eprintln!("[pipeline] Failed to set final status: {e}"),
        }

        let _ = persistence::remove_pipeline_stage_buffers(&ws, &conv_id);
        let _ = persistence::remove_pipeline_job_slots(&ws, &conv_id);
        let _ = persistence::remove_abort_flag(&ws, &conv_id);
    });

    Ok(detail)
}

#[tauri::command]
pub async fn get_pipeline_state(
    workspace_path: String,
    conversation_id: String,
) -> Result<Option<PipelineState>, String> {
    let mut state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?;

    // Merge live SSE output text from in-memory buffers for stages that are
    // still running (their text field is empty because they haven't written a
    // plan file yet). This lets the frontend show accumulated output even
    // after the user navigated away and back.
    if let Some(ref mut s) = state {
        let live_texts =
            persistence::get_pipeline_stage_texts(&workspace_path, &conversation_id)?;
        for (i, text) in live_texts.into_iter().enumerate() {
            if let Some(stage) = s.stages.get_mut(i) {
                if stage.text.is_empty() && !text.is_empty() {
                    stage.text = text;
                }
            }
        }
    }

    Ok(state)
}
