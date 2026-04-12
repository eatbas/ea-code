pub(super) mod artifacts;
mod hydration;
mod reconstruction;

use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};
use crate::storage::{atomic_write, now_rfc3339, with_conversations_lock};

use self::hydration::hydrate_stage_text;
use self::reconstruction::reconstruct_pipeline_from_artifacts;
use super::paths::pipeline_file_path;

fn save_pipeline_state_unlocked(path: &std::path::Path, state: &PipelineState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Failed to serialise pipeline state: {error}"))?;
    atomic_write(path, &json)
}

pub fn save_pipeline_state(
    workspace_path: &str,
    conversation_id: &str,
    state: &PipelineState,
) -> Result<(), String> {
    with_conversations_lock(|| {
        let path = pipeline_file_path(workspace_path, conversation_id);
        save_pipeline_state_unlocked(&path, state)
    })
}

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
            .map_err(|error| format!("Failed to read pipeline state: {error}"))?;
        let mut state: PipelineState = serde_json::from_str(&data)
            .map_err(|error| format!("Failed to parse pipeline state: {error}"))?;

        if let Some(stage) = state
            .stages
            .iter_mut()
            .find(|stage| stage.stage_index == record.stage_index)
        {
            stage.status = record.status.clone();
            stage.text.clone_from(&record.text);
            stage.score_id.clone_from(&record.score_id);
            stage
                .provider_session_ref
                .clone_from(&record.provider_session_ref);
            stage.started_at.clone_from(&record.started_at);
            stage.finished_at.clone_from(&record.finished_at);
        } else {
            state.stages.push(record.clone());
            state.stages.sort_by_key(|stage| stage.stage_index);
        }

        save_pipeline_state_unlocked(&path, &state)
    })
}

pub fn load_pipeline_state(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Option<PipelineState>, String> {
    with_conversations_lock(|| {
        let path = pipeline_file_path(workspace_path, conversation_id);
        if !path.exists() {
            return Ok(reconstruct_pipeline_from_artifacts(
                workspace_path,
                conversation_id,
            ));
        }

        let data = match std::fs::read_to_string(&path) {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(reconstruct_pipeline_from_artifacts(
                    workspace_path,
                    conversation_id,
                ));
            }
            Err(error) => return Err(format!("Failed to read pipeline state: {error}")),
        };
        let mut state: PipelineState = serde_json::from_str(&data)
            .map_err(|error| format!("Failed to parse pipeline state: {error}"))?;
        hydrate_stage_text(workspace_path, conversation_id, &mut state);
        Ok(Some(state))
    })
}

pub fn mark_running_pipeline_stages_stopped(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    with_conversations_lock(|| {
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
            save_pipeline_state_unlocked(&path, &state)?;
        }

        Ok(())
    })
}
