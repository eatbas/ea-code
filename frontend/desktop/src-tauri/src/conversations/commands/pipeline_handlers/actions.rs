//! Pipeline command handlers: start, stop, state query, accept, and feedback.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration, Instant};

use crate::commands::api_health::symphony_base_url;
use crate::conversations::events::EVENT_CONVERSATION_STATUS;
use crate::conversations::pipeline_debug::emit_pipeline_debug;
use crate::http::symphony_client;
use crate::models::{
    AgentSelection, ConversationDetail, ConversationStatus, ConversationStatusEvent,
    ConversationSummary, PipelineStageRecord, PipelineState,
};
use crate::storage::now_rfc3339;

use super::super::super::persistence;
use super::super::super::pipeline;
use super::super::pipeline_orchestration::{
    begin_pipeline_task, determine_final_status, emit_final_status, ensure_merge_stage_record,
    load_pipeline_config, pipeline_cleanup, prepare_pipeline, prepare_pipeline_with_config,
    re_emit_completed_stages, run_coding_phase, run_merge_chain,
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
    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "Pipeline requested for prompt: {}",
            trimmed.replace('\n', " ")
        ),
    );

    let setup = prepare_pipeline_with_config(&workspace_path, &conversation_id, config)?;

    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let prompt_dir = format!("{conv_dir}/prompt");
    if let Err(e) = std::fs::create_dir_all(&prompt_dir) {
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("Failed to create prompt directory: {e}"),
        );
    }
    if let Err(e) = std::fs::write(format!("{prompt_dir}/prompt.md"), trimmed) {
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("Failed to save prompt: {e}"),
        );
    }

    let mut seed_stages = Vec::new();
    if let Some(orchestrator_index) = setup.indices.orchestrator {
        let orchestrator_label = setup
            .orchestrator_agent
            .as_ref()
            .map(|agent| format!("{} / {}", agent.provider, agent.model))
            .unwrap_or_default();
        seed_stages.push(PipelineStageRecord {
            stage_index: orchestrator_index,
            stage_name: "Prompt Enhancer".to_string(),
            agent_label: orchestrator_label,
            status: ConversationStatus::Running,
            text: String::new(),
            started_at: Some(now_rfc3339()),
            finished_at: None,
            score_id: None,
            provider_session_ref: None,
        });
    }
    let planner_start = setup.indices.orchestrator.map(|_| 1).unwrap_or(0);
    for planner_number in 0..setup.planner_count {
        let planner = &setup.planners[planner_number];
        seed_stages.push(PipelineStageRecord {
            stage_index: planner_start + planner_number,
            stage_name: format!("Planner {}", planner_number + 1),
            agent_label: format!("{} / {}", planner.provider, planner.model),
            status: if setup.orchestrator_agent.is_some() {
                ConversationStatus::Idle
            } else {
                ConversationStatus::Running
            },
            text: String::new(),
            started_at: if setup.orchestrator_agent.is_some() {
                None
            } else {
                Some(now_rfc3339())
            },
            finished_at: None,
            score_id: None,
            provider_session_ref: None,
        });
    }
    seed_stages.sort_by_key(|stage| stage.stage_index);
    let seed_state = PipelineState {
        user_prompt: trimmed.to_string(),
        pipeline_mode: "code".to_string(),
        stages: seed_stages,
        review_cycle: 1,
        enhanced_prompt: None,
    };
    if let Err(e) = persistence::save_pipeline_state(&workspace_path, &conversation_id, &seed_state)
    {
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("Failed to seed pipeline state: {e}"),
        );
    }

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let user_prompt = trimmed.to_string();

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };
        emit_pipeline_debug(
            &app_handle,
            &ws,
            &conv_id,
            "Pipeline background task started",
        );

        let conv_dir = format!("{ws}/.maestro/conversations/{conv_id}");
        let prompt_dir = format!("{conv_dir}/prompt");

        // Step 1: Run orchestrator if configured.
        let mut effective_prompt = user_prompt.clone();
        let mut enhanced_prompt_saved: Option<String> = None;

        if let Some(orchestrator_agent) = &setup.orchestrator_agent {
            emit_pipeline_debug(&app_handle, &ws, &conv_id, "Running orchestrator stage...");

            let orchestrator_index = setup.indices.orchestrator.unwrap_or(0);
            let orchestrator_slot = setup
                .score_id_slots
                .get(orchestrator_index)
                .cloned()
                .unwrap_or_default();
            let orchestrator_buf = setup
                .stage_buffers
                .get(orchestrator_index)
                .cloned()
                .unwrap_or_default();

            match pipeline::run_orchestrator(
                app_handle.clone(),
                conv_id.clone(),
                ws.clone(),
                user_prompt.clone(),
                orchestrator_agent.clone(),
                orchestrator_index,
                setup.abort.clone(),
                orchestrator_slot,
                orchestrator_buf,
            )
            .await
            {
                Ok(result) => {
                    effective_prompt = result.enhanced_prompt.clone();
                    enhanced_prompt_saved = Some(result.enhanced_prompt.clone());

                    // Save enhanced prompt to file.
                    let enhanced_path = format!("{prompt_dir}/prompt_enhanced.md");
                    if let Err(e) = std::fs::write(&enhanced_path, &result.enhanced_prompt) {
                        emit_pipeline_debug(
                            &app_handle,
                            &ws,
                            &conv_id,
                            format!("Failed to save enhanced prompt: {e}"),
                        );
                    }

                    // Rename conversation with the summary title.
                    match persistence::rename_conversation(&ws, &conv_id, &result.summary) {
                        Ok(updated_summary) => {
                            // Emit conversation_status event to update sidebar and header.
                            let _ = app_handle.emit(
                                EVENT_CONVERSATION_STATUS,
                                ConversationStatusEvent {
                                    conversation: updated_summary,
                                    message: None,
                                },
                            );
                            emit_pipeline_debug(
                                &app_handle,
                                &ws,
                                &conv_id,
                                format!("Conversation renamed to: {}", result.summary),
                            );
                        }
                        Err(e) => {
                            emit_pipeline_debug(
                                &app_handle,
                                &ws,
                                &conv_id,
                                format!("Failed to rename conversation: {e}"),
                            );
                        }
                    }
                }
                Err(e) => {
                    emit_pipeline_debug(
                        &app_handle,
                        &ws,
                        &conv_id,
                        format!("Orchestrator failed, using original prompt: {e}"),
                    );
                    // Fall back to original prompt.
                    effective_prompt = user_prompt.clone();

                    // Generate a fallback title from the first 4 words.
                    let fallback_title: String = user_prompt
                        .split_whitespace()
                        .take(4)
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !fallback_title.is_empty() {
                        if let Ok(updated_summary) =
                            persistence::rename_conversation(&ws, &conv_id, &fallback_title)
                        {
                            let _ = app_handle.emit(
                                EVENT_CONVERSATION_STATUS,
                                ConversationStatusEvent {
                                    conversation: updated_summary,
                                    message: None,
                                },
                            );
                        }
                    }
                }
            }
        }

        // Step 2: Save enhanced_prompt to pipeline state BEFORE running planners.
        if let Some(enhanced) = enhanced_prompt_saved.clone() {
            if let Ok(Some(mut state)) = persistence::load_pipeline_state(&ws, &conv_id) {
                state.enhanced_prompt = Some(enhanced);
                let _ = persistence::save_pipeline_state(&ws, &conv_id, &state);
            }
        }

        // Step 3: Run planners with the effective prompt.
        let planner_start = setup.indices.orchestrator.map(|_| 1).unwrap_or(0);
        let planner_result = pipeline::run_pipeline_planners(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            setup.planners,
            planner_start,
            effective_prompt.clone(),
            setup.abort.clone(),
            setup
                .score_id_slots
                .iter()
                .skip(planner_start)
                .take(setup.planner_count)
                .cloned()
                .collect(),
            None,
            setup
                .stage_buffers
                .iter()
                .skip(planner_start)
                .take(setup.planner_count)
                .cloned()
                .collect(),
        )
        .await;

        let merge_result = if planner_result.is_ok() && !setup.abort.load(Ordering::Acquire) {
            run_merge_chain(
                app_handle.clone(),
                conv_id.clone(),
                ws.clone(),
                setup.abort.clone(),
                setup.merge_agent,
                setup.indices.plan_merge,
                setup.planner_count,
                &setup.score_id_slots,
                &setup.stage_buffers,
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
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationSummary, String> {
    persistence::trigger_abort(&workspace_path, &conversation_id)?;
    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        "Stop requested from UI",
    );

    let deadline = Instant::now() + PIPELINE_STOP_WAIT_TIMEOUT;
    let client = symphony_client();
    let mut stopped_score_ids = std::collections::HashSet::new();

    loop {
        let mut score_ids = persistence::get_pipeline_score_ids(&workspace_path, &conversation_id)?;
        // Follow-up turns run outside the pipeline score-slot registry; their
        // live score id lives on the summary. Include it so a single Stop
        // click still asks Symphony to cancel the follow-up.
        if let Ok(detail) = persistence::get_conversation(&workspace_path, &conversation_id) {
            if let Some(active) = detail.summary.active_score_id {
                if !score_ids.iter().any(|id| id == &active) {
                    score_ids.push(active);
                }
            }
        }
        if !score_ids.is_empty() {
            emit_pipeline_debug(
                &app,
                &workspace_path,
                &conversation_id,
                format!(
                    "Stop pipeline inspecting {} active score id(s)",
                    score_ids.len()
                ),
            );
        }
        for score_id in score_ids {
            if stopped_score_ids.insert(score_id.clone()) {
                emit_pipeline_debug(
                    &app,
                    &workspace_path,
                    &conversation_id,
                    format!("Sending Symphony stop request for score {score_id}"),
                );
                if let Err(e) = send_symphony_stop_request(&client, &score_id).await {
                    eprintln!("[pipeline] {e}");
                    emit_pipeline_debug(
                        &app,
                        &workspace_path,
                        &conversation_id,
                        format!("Symphony stop request failed for {score_id}: {e}"),
                    );
                }
            }
        }

        if Instant::now() >= deadline {
            break;
        }

        sleep(STOP_WAIT_POLL_INTERVAL).await;
    }

    persistence::mark_running_pipeline_stages_stopped(&workspace_path, &conversation_id)?;
    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        "Marked running pipeline stages as stopped",
    );

    persistence::set_status(
        &workspace_path,
        &conversation_id,
        ConversationStatus::Stopped,
        None,
    )
}

#[tauri::command]
pub async fn get_pipeline_state(
    workspace_path: String,
    conversation_id: String,
) -> Result<Option<PipelineState>, String> {
    let mut state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?;

    if let Some(ref mut s) = state {
        let live_texts = persistence::get_pipeline_stage_texts(&workspace_path, &conversation_id)?;
        for (stage_index, text) in live_texts {
            if let Some(stage) = s
                .stages
                .iter_mut()
                .find(|stage| stage.stage_index == stage_index)
            {
                if stage.text.is_empty() && !text.is_empty() {
                    stage.text = text;
                }
            }
        }
    }

    Ok(state)
}

#[tauri::command]
pub async fn get_pipeline_debug_log(
    workspace_path: String,
    conversation_id: String,
) -> Result<String, String> {
    persistence::read_pipeline_debug_log(&workspace_path, &conversation_id)
}

#[tauri::command]
pub async fn accept_plan(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationDetail, String> {
    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    if detail.summary.status == ConversationStatus::Running {
        return Ok(detail);
    }

    if detail.summary.status != ConversationStatus::AwaitingReview {
        return Err("Plan can only be accepted when status is awaiting_review".to_string());
    }

    let setup = prepare_pipeline(&workspace_path, &conversation_id)?;
    let guard = begin_pipeline_task(&app, &workspace_path, &conversation_id)
        .ok_or("Conversation is already running".to_string())?;
    let running_detail = persistence::get_conversation(&workspace_path, &conversation_id)?;

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();

    tokio::spawn(async move {
        let _guard = guard;

        // Re-emit all completed planning stages (planners + merge) so the
        // frontend displays them after its state reset.
        re_emit_completed_stages(
            &app_handle,
            &conv_id,
            &ws,
            setup.indices.coder, // emit everything before the Coder
        );

        let (status, error) = run_coding_phase(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            &setup,
            None,
        )
        .await;

        emit_final_status(&app_handle, &ws, &conv_id, status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(running_detail)
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

        let merge_label = format!(
            "{} / {}",
            setup.merge_agent.provider, setup.merge_agent.model
        );
        ensure_merge_stage_record(&ws, &conv_id, setup.indices.plan_merge, &merge_label);

        let merge_slot = setup
            .score_id_slots
            .get(setup.indices.plan_merge)
            .cloned()
            .unwrap_or_default();
        let merge_buf = setup
            .stage_buffers
            .get(setup.indices.plan_merge)
            .cloned()
            .unwrap_or_default();

        re_emit_completed_stages(&app_handle, &conv_id, &ws, setup.indices.plan_merge);

        let merge_result = pipeline::run_plan_merge_with_feedback(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            setup.abort.clone(),
            merge_slot,
            merge_buf,
            setup.indices.plan_merge,
            setup.planner_count,
            session_ref,
            setup.merge_agent,
            user_feedback,
        )
        .await;

        let (status, error) = if setup.abort.load(Ordering::Acquire) {
            (ConversationStatus::Stopped, None)
        } else {
            match &merge_result {
                Ok(_) => (ConversationStatus::AwaitingReview, None),
                Err((_, e)) => (ConversationStatus::Failed, Some(e.clone())),
            }
        };

        emit_final_status(&app_handle, &ws, &conv_id, status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}
