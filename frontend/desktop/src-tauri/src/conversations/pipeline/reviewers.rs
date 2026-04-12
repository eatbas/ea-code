use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{ConversationStatus, PipelineAgent, PipelineStageRecord};
use crate::storage::now_rfc3339;

use super::prompts::{agent_label, build_reviewer_prompt};
use super::stage_runner::{emit_stage_record, run_stage, StageConfig};

/// Run all reviewers in parallel. Each reviewer resumes the corresponding
/// planner's session to retain plan context, then uses git tools to review
/// the coder's changes.
pub async fn run_pipeline_reviewers(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    reviewers: Vec<PipelineAgent>,
    abort: Arc<AtomicBool>,
    score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    previous_stages: Option<Vec<PipelineStageRecord>>,
    stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
    planner_stages: &[PipelineStageRecord],
    reviewer_start_index: usize,
    review_dir_override: Option<String>,
    stage_name_suffix: Option<String>,
) -> Result<(), String> {
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let review_dir = review_dir_override.unwrap_or_else(|| format!("{conv_dir}/review"));
    let plan_merged_path = format!("{conv_dir}/plan_merged/plan_merged.md");
    let suffix = stage_name_suffix.unwrap_or_default();

    std::fs::create_dir_all(&review_dir)
        .map_err(|e| format!("Failed to create review directory: {e}"))?;

    let mut spawned_indices: Vec<usize> = Vec::new();
    let mut handles = Vec::new();

    for (i, reviewer_agent) in reviewers.into_iter().enumerate() {
        let stage_idx = reviewer_start_index + i;
        let reviewer_number = i + 1;
        let label = agent_label(&reviewer_agent);

        let already_completed = previous_stages
            .as_ref()
            .and_then(|s| s.iter().find(|st| st.stage_index == stage_idx))
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false);

        if already_completed {
            if let Some(record) = previous_stages
                .as_ref()
                .and_then(|s| s.iter().find(|st| st.stage_index == stage_idx))
            {
                let _ = emit_stage_record(
                    &app,
                    &conversation_id,
                    record,
                    if record.text.is_empty() {
                        None
                    } else {
                        Some(record.text.clone())
                    },
                );
            }
            continue;
        }

        let planner_session_ref = match planner_stages.get(i) {
            Some(stage) => match stage
                .provider_session_ref
                .clone()
                .filter(|value| !value.is_empty())
            {
                Some(session_ref) => session_ref,
                None => {
                    let failed_record = PipelineStageRecord::failed(
                        stage_idx,
                        format!("Reviewer {reviewer_number}{suffix}"),
                        label.clone(),
                        Some(now_rfc3339()),
                    );
                    let _ = crate::conversations::persistence::update_pipeline_stage(
                        &workspace_path,
                        &conversation_id,
                        &failed_record,
                    );
                    let _ = emit_stage_record(&app, &conversation_id, &failed_record, None);
                    return Err(format!(
                        "Reviewer {reviewer_number}: Planner {reviewer_number} is missing a provider session ref",
                    ));
                }
            },
            None => {
                let failed_record = PipelineStageRecord::failed(
                    stage_idx,
                    format!("Reviewer {reviewer_number}{suffix}"),
                    label.clone(),
                    Some(now_rfc3339()),
                );
                let _ = crate::conversations::persistence::update_pipeline_stage(
                    &workspace_path,
                    &conversation_id,
                    &failed_record,
                );
                let _ = emit_stage_record(&app, &conversation_id, &failed_record, None);
                return Err(format!(
                    "Reviewer {reviewer_number}: missing matching Planner {reviewer_number} stage",
                ));
            }
        };

        let job_slot = score_id_slots.get(i).cloned().unwrap_or_default();
        let out_buf = stage_buffers.get(i).cloned().unwrap_or_default();
        let app_c = app.clone();
        let conv_id = conversation_id.clone();
        let ws = workspace_path.clone();
        let dir = review_dir.clone();
        let plan_path = plan_merged_path.clone();
        let abort_c = abort.clone();
        let sfx = suffix.clone();

        spawned_indices.push(stage_idx);
        handles.push(tokio::spawn(async move {
            run_stage(
                app_c,
                conv_id,
                ws,
                StageConfig {
                    stage_index: stage_idx,
                    stage_name: format!("Reviewer {reviewer_number}{sfx}"),
                    provider: reviewer_agent.provider,
                    model: reviewer_agent.model,
                    prompt: build_reviewer_prompt(reviewer_number, &plan_path, &dir),
                    file_to_watch: format!("{dir}/Review-{reviewer_number}.md"),
                    mode: "resume",
                    provider_session_ref: Some(planner_session_ref),
                    failure_message: format!("Reviewer {reviewer_number} did not produce a review"),
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
    let mut errors = Vec::new();

    for (result_idx, result) in results.into_iter().enumerate() {
        let stage_idx = spawned_indices[result_idx];
        match result {
            Ok(Ok(_record)) => {}
            Ok(Err((_record, e))) => {
                errors.push(format!(
                    "Reviewer {}: {e}",
                    stage_idx - reviewer_start_index + 1
                ));
            }
            Err(e) => {
                errors.push(format!(
                    "Reviewer {} panicked: {e}",
                    stage_idx - reviewer_start_index + 1
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
