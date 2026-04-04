//! The redo_review_pipeline command — re-runs the review cycle
//! (Reviewers -> Review Merge -> Code Fixer) with incrementing folder names.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{ConversationDetail, ConversationStatus, PipelineStageRecord};

use super::super::super::persistence;
use super::super::super::pipeline;
use super::super::pipeline_orchestration::{
    begin_pipeline_task, emit_final_status, ensure_stage_record, pipeline_cleanup, prepare_pipeline,
};

#[tauri::command]
pub async fn redo_review_pipeline(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationDetail, String> {
    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;

    let mut state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    let setup = prepare_pipeline(&workspace_path, &conversation_id)?;

    // Determine the new review cycle number.
    let current_cycle = state.review_cycle;
    let new_cycle = current_cycle + 1;

    // Build directory paths for the new cycle.
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let review_dir = format!("{conv_dir}/review_{new_cycle}");
    let review_merged_dir = format!("{conv_dir}/review_merged_{new_cycle}");
    let code_fixer_dir = format!("{conv_dir}/code_fixer_{new_cycle}");
    let review_merged_path = format!("{review_merged_dir}/review_merged.md");
    let cycle_suffix = format!(" (Cycle {new_cycle})");

    // Compute new stage indices starting after the last existing stage.
    let max_existing_index = state
        .stages
        .iter()
        .map(|s| s.stage_index)
        .max()
        .unwrap_or(0);
    let reviewer_start = max_existing_index + 1;
    let reviewer_count = setup.reviewer_count;
    let review_merge_index = reviewer_start + reviewer_count;
    let code_fixer_index = review_merge_index + 1;

    // Retrieve the coder session ref (needed for Code Fixer to resume).
    let coder_ref = state
        .stages
        .iter()
        .find(|s| s.stage_name == "Coder")
        .and_then(|s| s.provider_session_ref.clone())
        .unwrap_or_default();

    if coder_ref.is_empty() {
        return Err("No coder session ref available for re-do review".to_string());
    }

    // Collect planner stages for reviewer session resumption.
    let planner_stages: Vec<PipelineStageRecord> = state
        .stages
        .iter()
        .filter(|s| s.stage_name.starts_with("Planner"))
        .cloned()
        .collect();

    // Update the review cycle counter and save immediately.
    state.review_cycle = new_cycle;
    persistence::save_pipeline_state(&workspace_path, &conversation_id, &state)?;

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };

        // Re-emit all existing completed stages so the frontend sees them.
        re_emit_all_stages(&app_handle, &conv_id, &ws);

        // Use the setup's abort flag (already registered with persistence).
        let abort = setup.abort.clone();

        // --- Reviewers ---
        let reviewer_slots: Vec<_> = (0..reviewer_count)
            .map(|_| Arc::new(std::sync::Mutex::new(None::<String>)))
            .collect();
        let reviewer_bufs: Vec<_> = (0..reviewer_count)
            .map(|_| Arc::new(std::sync::Mutex::new(String::new())))
            .collect();

        for (i, reviewer) in setup.reviewers.iter().enumerate() {
            let label = format!("{} / {}", reviewer.provider, reviewer.model);
            ensure_stage_record(
                &ws,
                &conv_id,
                reviewer_start + i,
                &format!("Reviewer {}{}", i + 1, cycle_suffix),
                &label,
            );
        }

        let reviewer_result = pipeline::run_pipeline_reviewers(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            setup.reviewers.clone(),
            abort.clone(),
            reviewer_slots,
            None,
            reviewer_bufs,
            &planner_stages,
            reviewer_start,
            Some(review_dir.clone()),
            Some(cycle_suffix.clone()),
        )
        .await;

        if abort.load(Ordering::Acquire) {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Stopped,
                None,
            );
            pipeline_cleanup(&ws, &conv_id);
            return;
        }

        if let Err(e) = reviewer_result {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Failed,
                Some(e),
            );
            pipeline_cleanup(&ws, &conv_id);
            return;
        }

        // --- Review Merge ---
        let Some(review_merge_agent) = setup.reviewers.first().cloned() else {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Failed,
                Some("No reviewer available for Review Merge".to_string()),
            );
            pipeline_cleanup(&ws, &conv_id);
            return;
        };

        // Load the first reviewer's session ref from the new cycle stages.
        let loaded = persistence::load_pipeline_state(&ws, &conv_id)
            .ok()
            .flatten();
        let reviewer_session_ref = loaded
            .as_ref()
            .and_then(|s| {
                s.stages
                    .iter()
                    .find(|st| st.stage_index == reviewer_start)
                    .and_then(|st| st.provider_session_ref.clone())
            })
            .unwrap_or_default();

        if reviewer_session_ref.is_empty() {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Failed,
                Some("No reviewer session ref for Review Merge".to_string()),
            );
            pipeline_cleanup(&ws, &conv_id);
            return;
        }

        let rm_label = format!(
            "{} / {}",
            review_merge_agent.provider, review_merge_agent.model
        );
        let rm_stage_name = format!("Review Merge{cycle_suffix}");
        ensure_stage_record(&ws, &conv_id, review_merge_index, &rm_stage_name, &rm_label);

        let rm_slot = Arc::new(std::sync::Mutex::new(None::<String>));
        let rm_buf = Arc::new(std::sync::Mutex::new(String::new()));

        let rm_result = pipeline::run_review_merge(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            abort.clone(),
            rm_slot,
            rm_buf,
            review_merge_index,
            reviewer_count,
            reviewer_session_ref,
            review_merge_agent,
            Some(review_dir.clone()),
            Some(review_merged_dir.clone()),
            Some(rm_stage_name),
        )
        .await;

        if abort.load(Ordering::Acquire) {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Stopped,
                None,
            );
            pipeline_cleanup(&ws, &conv_id);
            return;
        }

        if let Err((_, e)) = rm_result {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Failed,
                Some(e),
            );
            pipeline_cleanup(&ws, &conv_id);
            return;
        }

        // --- Code Fixer ---
        let fixer_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
        let fixer_stage_name = format!("Code Fixer{cycle_suffix}");
        ensure_stage_record(
            &ws,
            &conv_id,
            code_fixer_index,
            &fixer_stage_name,
            &fixer_label,
        );

        let fixer_slot = Arc::new(std::sync::Mutex::new(None::<String>));
        let fixer_buf = Arc::new(std::sync::Mutex::new(String::new()));

        let fixer_result = pipeline::run_code_fixer(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            abort.clone(),
            fixer_slot,
            fixer_buf,
            code_fixer_index,
            coder_ref,
            setup.coder.clone(),
            Some(code_fixer_dir),
            Some(review_merged_path),
            Some(fixer_stage_name),
        )
        .await;

        if abort.load(Ordering::Acquire) {
            emit_final_status(
                &app_handle,
                &ws,
                &conv_id,
                ConversationStatus::Stopped,
                None,
            );
        } else {
            match fixer_result {
                Ok(_) => emit_final_status(
                    &app_handle,
                    &ws,
                    &conv_id,
                    ConversationStatus::Completed,
                    None,
                ),
                Err((_, e)) => emit_final_status(
                    &app_handle,
                    &ws,
                    &conv_id,
                    ConversationStatus::Failed,
                    Some(e),
                ),
            }
        }

        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}

/// Re-emit all saved stages (regardless of index) so the frontend sees
/// the full history including previous review cycles.
fn re_emit_all_stages(app: &AppHandle, conv_id: &str, ws: &str) {
    use crate::conversations::events::EVENT_PIPELINE_STAGE_STATUS;
    use crate::models::PipelineStageStatusEvent;
    use tauri::Emitter;

    if let Ok(Some(saved)) = persistence::load_pipeline_state(ws, conv_id) {
        for stage in &saved.stages {
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
