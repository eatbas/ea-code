use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration, Instant};

use crate::commands::api_health::symphony_base_url;
use crate::http::symphony_client;
use crate::models::{
    AgentSelection, ConversationDetail, ConversationStatus, ConversationStatusEvent,
    ConversationSummary, PipelineState,
};

use super::super::events::EVENT_CONVERSATION_STATUS;
use super::super::persistence;
use super::super::pipeline;
use super::pipeline_orchestration::{
    begin_pipeline_task, determine_final_status, emit_final_status, ensure_merge_stage_record,
    load_pipeline_config, pipeline_cleanup, prepare_pipeline, prepare_pipeline_with_config,
    re_emit_completed_stages, run_merge_chain,
};

const STOP_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const PIPELINE_STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(3);

async fn send_symphony_stop_request(
    client: &reqwest::Client,
    score_id: &str,
) -> Result<(), String> {
    let url = format!("{}/v1/chat/{score_id}/stop", symphony_base_url());
    let response = client
        .post(url)
        .send()
        .await
        .map_err(|error| format!("Failed to stop symphony score {score_id}: {error}"))?;
    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(format!(
        "Failed to stop symphony score {score_id}: HTTP {status} — {body}"
    ))
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

    let config = load_pipeline_config()?;

    let agent = AgentSelection {
        provider: config.merge_agent.provider.clone(),
        model: config.merge_agent.model.clone(),
    };

    let detail = persistence::create_conversation(&workspace_path, agent, Some(trimmed))?;
    let conversation_id = detail.summary.id.clone();

    let setup = prepare_pipeline_with_config(&workspace_path, &conversation_id, config)?;

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let user_prompt = trimmed.to_string();

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };

        let planner_result = pipeline::run_pipeline_planners(
            app_handle.clone(), conv_id.clone(), ws.clone(),
            setup.planners, user_prompt, setup.abort.clone(),
            setup.score_id_slots[..setup.planner_count].to_vec(), None,
            setup.stage_buffers[..setup.planner_count].to_vec(),
        )
        .await;

        let merge_result = if planner_result.is_ok()
            && !setup.abort.load(Ordering::Acquire)
        {
            run_merge_chain(
                app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                setup.merge_agent, setup.planner_count,
                &setup.score_id_slots, &setup.stage_buffers,
            )
            .await
        } else {
            None
        };

        let (status, error) = determine_final_status(&setup.abort, &planner_result, &merge_result);
        emit_final_status(&app_handle, &ws, &conv_id, status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}

#[tauri::command]
pub async fn stop_pipeline(
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    persistence::trigger_abort(&workspace_path, &conversation_id)?;

    let deadline = Instant::now() + PIPELINE_STOP_WAIT_TIMEOUT;
    let client = symphony_client();
    let mut stopped_score_ids = std::collections::HashSet::new();

    loop {
        let score_ids = persistence::get_pipeline_score_ids(&workspace_path, &conversation_id)?;
        for score_id in score_ids {
            if stopped_score_ids.insert(score_id.clone()) {
                if let Err(e) = send_symphony_stop_request(&client, &score_id).await {
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

    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    let setup = prepare_pipeline(&workspace_path, &conversation_id)?;

    let user_prompt = state.user_prompt;
    let previous_stages = state.stages;

    let all_planners_done = previous_stages
        .iter()
        .take(setup.planner_count)
        .all(|s| s.status == ConversationStatus::Completed);
    let merge_needs_run = previous_stages
        .get(setup.planner_count)
        .map(|s| s.status != ConversationStatus::Completed)
        .unwrap_or(true);

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };

        let planner_result = if all_planners_done {
            re_emit_completed_stages(&app_handle, &conv_id, &ws, setup.planner_count);
            Ok(())
        } else {
            pipeline::run_pipeline_planners(
                app_handle.clone(), conv_id.clone(), ws.clone(),
                setup.planners, user_prompt, setup.abort.clone(),
                setup.score_id_slots[..setup.planner_count].to_vec(), Some(previous_stages),
                setup.stage_buffers[..setup.planner_count].to_vec(),
            )
            .await
        };

        let merge_result = if planner_result.is_ok()
            && merge_needs_run
            && !setup.abort.load(Ordering::Acquire)
        {
            run_merge_chain(
                app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                setup.merge_agent, setup.planner_count,
                &setup.score_id_slots, &setup.stage_buffers,
            )
            .await
        } else {
            None
        };

        let (status, error) = determine_final_status(&setup.abort, &planner_result, &merge_result);
        emit_final_status(&app_handle, &ws, &conv_id, status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}

#[tauri::command]
pub async fn get_pipeline_state(
    workspace_path: String,
    conversation_id: String,
) -> Result<Option<PipelineState>, String> {
    let mut state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?;

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

#[tauri::command]
pub async fn accept_plan(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    if detail.summary.status != ConversationStatus::AwaitingReview {
        return Err("Plan can only be accepted when status is awaiting_review".to_string());
    }
    let summary = persistence::set_status(
        &workspace_path,
        &conversation_id,
        ConversationStatus::Completed,
        None,
    )?;
    let _ = app.emit(
        EVENT_CONVERSATION_STATUS,
        ConversationStatusEvent {
            conversation: summary.clone(),
            message: None,
        },
    );
    Ok(summary)
}

#[tauri::command]
pub async fn send_plan_edit_feedback(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    feedback: String,
) -> Result<ConversationDetail, String> {
    let trimmed = feedback.trim();
    if trimmed.is_empty() {
        return Err("Feedback must not be empty".to_string());
    }

    let state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    let merge_stage = state
        .stages
        .iter()
        .find(|s| s.stage_name == "Plan Merge")
        .ok_or("No Plan Merge stage found")?;
    let session_ref = merge_stage
        .provider_session_ref
        .clone()
        .ok_or("No provider session ref for Plan Merge stage")?;

    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    let setup = prepare_pipeline(&workspace_path, &conversation_id)?;

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let user_feedback = trimmed.to_string();

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };

        let merge_label = format!("{} / {}", setup.merge_agent.provider, setup.merge_agent.model);
        ensure_merge_stage_record(&ws, &conv_id, setup.planner_count, &merge_label);

        let merge_slot = setup.score_id_slots.get(setup.planner_count).cloned().unwrap_or_default();
        let merge_buf = setup.stage_buffers.get(setup.planner_count).cloned().unwrap_or_default();

        re_emit_completed_stages(&app_handle, &conv_id, &ws, setup.planner_count);

        let merge_result = pipeline::run_plan_merge_with_feedback(
            app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
            merge_slot, merge_buf, setup.planner_count, session_ref,
            setup.merge_agent, user_feedback,
        )
        .await;

        let final_status = if setup.abort.load(Ordering::Acquire) {
            ConversationStatus::Stopped
        } else {
            match &merge_result {
                Ok(_) => ConversationStatus::AwaitingReview,
                Err(_) => ConversationStatus::Failed,
            }
        };
        let error = merge_result.err().map(|(_, e)| e);

        emit_final_status(&app_handle, &ws, &conv_id, final_status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}
