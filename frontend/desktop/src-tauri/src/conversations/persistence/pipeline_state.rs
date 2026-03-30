use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};
use crate::storage::{atomic_write, now_rfc3339, with_conversations_lock};

use super::paths::{conversation_dir, pipeline_file_path};

pub fn save_pipeline_state(
    workspace_path: &str,
    conversation_id: &str,
    state: &PipelineState,
) -> Result<(), String> {
    let path = pipeline_file_path(workspace_path, conversation_id);
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialise pipeline state: {e}"))?;
    atomic_write(&path, &json)
}

/// Atomically update a single stage inside pipeline.json. Called when an
/// individual planner finishes so the state is visible if the user navigates
/// away before the whole pipeline completes.
pub fn update_pipeline_stage(
    workspace_path: &str,
    conversation_id: &str,
    record: &PipelineStageRecord,
) -> Result<(), String> {
    with_conversations_lock(|| {
        let path = pipeline_file_path(workspace_path, conversation_id);
        if !path.exists() {
            return Ok(());
        }
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read pipeline state: {e}"))?;
        let mut state: PipelineState = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse pipeline state: {e}"))?;

        if let Some(stage) = state.stages.get_mut(record.stage_index) {
            stage.status = record.status.clone();
            stage.job_id.clone_from(&record.job_id);
            stage
                .provider_session_ref
                .clone_from(&record.provider_session_ref);
            stage.started_at.clone_from(&record.started_at);
            stage.finished_at.clone_from(&record.finished_at);
        }

        save_pipeline_state(workspace_path, conversation_id, &state)
    })
}

/// Fills in the `text` field of each stage from the corresponding plan file
/// on disk. Also corrects stale "Running" status when the plan file already
/// exists.
fn hydrate_stage_text(workspace_path: &str, conversation_id: &str, state: &mut PipelineState) {
    let conv_dir = conversation_dir(workspace_path, conversation_id);
    let plan_dir = conv_dir.join("plan");
    let merged_file = conv_dir.join("plan_merged").join("plan_merged.md");

    for stage in &mut state.stages {
        let plan_file = plan_dir.join(format!("Plan-{}.md", stage.stage_index + 1));
        if let Ok(contents) = std::fs::read_to_string(&plan_file) {
            stage.text = contents;
            if stage.status == ConversationStatus::Running {
                stage.status = ConversationStatus::Completed;
            }
        }

        if stage.stage_name == "Plan Merge" {
            if let Ok(contents) = std::fs::read_to_string(&merged_file) {
                stage.text = contents;
                if stage.status == ConversationStatus::Running {
                    stage.status = ConversationStatus::Completed;
                }
            }
        }
    }

    // If the merged plan file exists on disk but no Plan Merge stage record
    // is present, insert the stage so the frontend can display it.
    let has_merge_stage = state.stages.iter().any(|s| s.stage_name == "Plan Merge");
    if !has_merge_stage {
        if let Ok(contents) = std::fs::read_to_string(&merged_file) {
            state.stages.push(PipelineStageRecord {
                stage_index: state.stages.len(),
                stage_name: "Plan Merge".to_string(),
                agent_label: String::new(),
                status: ConversationStatus::Completed,
                text: contents,
                started_at: None,
                finished_at: None,
                job_id: None,
                provider_session_ref: None,
            });
        }
    }
}

/// Reconstructs a PipelineState from disk artifacts when pipeline.json is
/// missing (e.g. crash before first save in older versions).
fn reconstruct_pipeline_from_artifacts(
    workspace_path: &str,
    conversation_id: &str,
) -> Option<PipelineState> {
    let conv_dir = conversation_dir(workspace_path, conversation_id);
    let prompt_path = conv_dir.join("prompt").join("prompt.md");
    let plan_dir = conv_dir.join("plan");

    let user_prompt = std::fs::read_to_string(&prompt_path).ok()?;

    let mut stages: Vec<PipelineStageRecord> = Vec::new();
    if plan_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&plan_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(rest) = name_str.strip_prefix("Plan-") {
                    if let Some(num_str) = rest.strip_suffix(".md") {
                        if let Ok(n) = num_str.parse::<usize>() {
                            let text =
                                std::fs::read_to_string(entry.path()).unwrap_or_default();
                            stages.push(PipelineStageRecord {
                                stage_index: n - 1,
                                stage_name: format!("Planner {n}"),
                                agent_label: String::new(),
                                status: ConversationStatus::Completed,
                                text,
                                started_at: None,
                                finished_at: None,
                                job_id: None,
                                provider_session_ref: None,
                            });
                        }
                    }
                }
            }
        }
    }

    if stages.is_empty() {
        stages.push(PipelineStageRecord {
            stage_index: 0,
            stage_name: "Planner 1".to_string(),
            agent_label: String::new(),
            status: ConversationStatus::Failed,
            text: String::new(),
            started_at: None,
            finished_at: None,
            job_id: None,
            provider_session_ref: None,
        });
    }

    stages.sort_by_key(|s| s.stage_index);

    let merged_file = conv_dir.join("plan_merged").join("plan_merged.md");
    if merged_file.exists() {
        let text = std::fs::read_to_string(&merged_file).unwrap_or_default();
        stages.push(PipelineStageRecord {
            stage_index: stages.len(),
            stage_name: "Plan Merge".to_string(),
            agent_label: String::new(),
            status: ConversationStatus::Completed,
            text,
            started_at: None,
            finished_at: None,
            job_id: None,
            provider_session_ref: None,
        });
    }

    let state = PipelineState {
        user_prompt,
        pipeline_mode: "code".to_string(),
        stages,
    };

    if let Err(e) = save_pipeline_state(workspace_path, conversation_id, &state) {
        eprintln!("[pipeline] Failed to persist reconstructed pipeline state: {e}");
    }

    Some(state)
}

pub fn load_pipeline_state(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Option<PipelineState>, String> {
    let path = pipeline_file_path(workspace_path, conversation_id);
    if !path.exists() {
        return Ok(reconstruct_pipeline_from_artifacts(
            workspace_path,
            conversation_id,
        ));
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read pipeline state: {e}"))?;
    let mut state: PipelineState =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse pipeline state: {e}"))?;
    hydrate_stage_text(workspace_path, conversation_id, &mut state);
    Ok(Some(state))
}

pub fn mark_running_pipeline_stages_stopped(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let path = pipeline_file_path(workspace_path, conversation_id);
    if !path.exists() {
        return Ok(());
    }

    let data = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read pipeline state {}: {error}", path.display()))?;
    let mut state: PipelineState = serde_json::from_str(&data)
        .map_err(|error| format!("Failed to parse pipeline state {}: {error}", path.display()))?;

    let mut changed = false;
    for stage in &mut state.stages {
        if stage.status == ConversationStatus::Running {
            stage.status = ConversationStatus::Stopped;
            stage.finished_at = Some(now_rfc3339());
            changed = true;
        }
    }

    if changed {
        save_pipeline_state(workspace_path, conversation_id, &state)?;
    }

    Ok(())
}
