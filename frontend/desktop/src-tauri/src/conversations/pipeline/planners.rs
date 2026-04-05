use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{ConversationStatus, PipelineAgent, PipelineStageRecord, PipelineState};
use crate::storage::now_rfc3339;

use super::prompts::{agent_label, build_planner_prompt};
use super::stage_runner::{emit_stage_status, run_stage, StageConfig};
use crate::conversations::pipeline_debug::emit_pipeline_debug;

/// Run all planners in parallel. Returns when all planners have completed.
pub async fn run_pipeline_planners(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    planners: Vec<PipelineAgent>,
    planner_start_index: usize,
    user_prompt: String,
    abort: Arc<AtomicBool>,
    score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    previous_stages: Option<Vec<PipelineStageRecord>>,
    stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
) -> Result<(), String> {
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "Initialising planner phase with {} planner(s)",
            planners.len()
        ),
    );

    // Create the plan folder for planner outputs.
    let plan_dir = format!("{conv_dir}/plan");
    std::fs::create_dir_all(&plan_dir)
        .map_err(|e| format!("Failed to create plan directory: {e}"))?;

    // Build the initial pipeline state.
    let initial_stages: Vec<PipelineStageRecord> = planners
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let stage_index = planner_start_index + i;
            let previous_stage = previous_stages
                .as_ref()
                .and_then(|s| s.iter().find(|st| st.stage_index == stage_index));
            let already_completed = previous_stage
                .map(|s| s.status == ConversationStatus::Completed)
                .unwrap_or(false);

            if already_completed {
                previous_stage.unwrap().clone()
            } else {
                PipelineStageRecord {
                    stage_index,
                    stage_name: format!("Planner {}", i + 1),
                    agent_label: agent_label(a),
                    status: ConversationStatus::Running,
                    text: String::new(),
                    started_at: Some(now_rfc3339()),
                    finished_at: None,
                    score_id: None,
                    provider_session_ref: None,
                }
            }
        })
        .collect();
    // Load existing state to preserve orchestrator data.
    let existing = super::super::persistence::load_pipeline_state(&workspace_path, &conversation_id)
        .ok()
        .flatten();

    let mut all_stages = existing
        .as_ref()
        .map(|s| s.stages.iter().filter(|st| !st.stage_name.starts_with("Planner")).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    all_stages.extend(initial_stages);
    all_stages.sort_by_key(|s| s.stage_index);

    let initial_state = PipelineState {
        user_prompt: existing.as_ref().map(|s| s.user_prompt.clone()).unwrap_or(user_prompt.clone()),
        pipeline_mode: "code".to_string(),
        stages: all_stages,
        review_cycle: existing.as_ref().map(|s| s.review_cycle).unwrap_or(1),
        enhanced_prompt: existing.and_then(|s| s.enhanced_prompt),
    };
    if let Err(e) = super::super::persistence::save_pipeline_state(
        &workspace_path,
        &conversation_id,
        &initial_state,
    ) {
        eprintln!("[pipeline] Failed to save initial pipeline state: {e}");
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("Failed to save initial pipeline state: {e}"),
        );
    }

    let planner_count = planners.len();
    let mut spawned_indices: Vec<usize> = Vec::new();
    let mut handles = Vec::new();
    let mut completed_records: Vec<PipelineStageRecord> = Vec::new();

    for (i, planner_agent) in planners.into_iter().enumerate() {
        let stage_index = planner_start_index + i;
        let previous_stage = previous_stages
            .as_ref()
            .and_then(|s| s.iter().find(|st| st.stage_index == stage_index));
        let already_completed = previous_stage
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false);

        if already_completed {
            if let Some(record) = previous_stage {
                let _ = emit_stage_status(
                    &app,
                    &conversation_id,
                    stage_index,
                    &record.stage_name,
                    record.status.clone(),
                    &record.agent_label,
                    if record.text.is_empty() {
                        None
                    } else {
                        Some(record.text.clone())
                    },
                );
                completed_records.push(record.clone());
            }
            continue;
        }

        let resume_ref = previous_stage.and_then(|s| s.provider_session_ref.clone());

        let job_slot = score_id_slots.get(i).cloned().unwrap_or_default();
        let out_buf = stage_buffers.get(i).cloned().unwrap_or_default();
        let app_c = app.clone();
        let conv_id = conversation_id.clone();
        let ws = workspace_path.clone();
        let dir = plan_dir.clone();
        let prompt_text = user_prompt.clone();
        let abort_c = abort.clone();
        let planner_number = i + 1;
        let label = agent_label(&planner_agent);
        let mode = if resume_ref.is_some() {
            "resume"
        } else {
            "new"
        };
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!(
                "Planner {planner_number} queued with {} / {} in {mode} mode",
                planner_agent.provider, planner_agent.model,
            ),
        );

        spawned_indices.push(stage_index);
        handles.push(tokio::spawn(async move {
            run_stage(
                app_c,
                conv_id,
                ws,
                StageConfig {
                    stage_index,
                    stage_name: format!("Planner {planner_number}"),
                    provider: planner_agent.provider,
                    model: planner_agent.model,
                    prompt: build_planner_prompt(planner_number, &dir, &prompt_text),
                    file_to_watch: format!("{dir}/Plan-{planner_number}.md"),
                    mode,
                    provider_session_ref: resume_ref,
                    failure_message: format!("Planner {planner_number} did not produce a plan"),
                    agent_label: label,
                    file_required: true,
                },
                abort_c,
                job_slot,
                out_buf,
            )
            .await
        }));
    }

    let results = futures::future::join_all(handles).await;
    let mut stage_records: Vec<PipelineStageRecord> = completed_records;
    stage_records.reserve(planner_count);
    let mut errors = Vec::new();

    for (result_idx, result) in results.into_iter().enumerate() {
        let stage_idx = spawned_indices[result_idx];
        match result {
            Ok(Ok(record)) => stage_records.push(record),
            Ok(Err((record, e))) => {
                stage_records.push(record);
                errors.push(format!("Planner {}: {e}", stage_idx + 1));
            }
            Err(e) => {
                stage_records.push(PipelineStageRecord::failed(
                    stage_idx,
                    format!("Planner {}", stage_idx + 1),
                    String::new(),
                    None,
                ));
                errors.push(format!("Planner {} panicked: {e}", stage_idx + 1));
            }
        }
    }

    stage_records.sort_by_key(|s| s.stage_index);

    // Load existing state to preserve orchestrator data when merging.
    let existing_final = super::super::persistence::load_pipeline_state(&workspace_path, &conversation_id)
        .ok()
        .flatten();

    let mut all_final_stages = existing_final
        .as_ref()
        .map(|s| s.stages.iter().filter(|st| !st.stage_name.starts_with("Planner")).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    all_final_stages.extend(stage_records);
    all_final_stages.sort_by_key(|s| s.stage_index);

    let state = PipelineState {
        user_prompt: existing_final.as_ref().map(|s| s.user_prompt.clone()).unwrap_or(user_prompt),
        pipeline_mode: "code".to_string(),
        stages: all_final_stages,
        review_cycle: existing_final.as_ref().map(|s| s.review_cycle).unwrap_or(1),
        enhanced_prompt: existing_final.and_then(|s| s.enhanced_prompt),
    };
    if let Err(e) =
        super::super::persistence::save_pipeline_state(&workspace_path, &conversation_id, &state)
    {
        eprintln!("[pipeline] Failed to save pipeline state: {e}");
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("Failed to save planner phase state: {e}"),
        );
    }

    if errors.is_empty() {
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            "Planner phase completed without errors",
        );
        Ok(())
    } else {
        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!("Planner phase completed with errors: {}", errors.join("; ")),
        );
        Err(errors.join("; "))
    }
}
