use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};

use super::super::paths::conversation_dir;
use super::artifacts::artifact_path_for_stage;

fn hydrate_from_file(stage: &mut PipelineStageRecord, path: &std::path::Path) {
    if let Ok(contents) = std::fs::read_to_string(path) {
        stage.text = contents;
        if stage.status == ConversationStatus::Running {
            stage.status = ConversationStatus::Completed;
        }
        if stage.finished_at.is_none() {
            stage.finished_at = file_modified_rfc3339(path).or_else(|| stage.started_at.clone());
        }
    }
}

fn file_modified_rfc3339(path: &std::path::Path) -> Option<String> {
    let modified_at = std::fs::metadata(path).ok()?.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified_at.into();
    Some(datetime.to_rfc3339())
}

pub(super) fn hydrate_stage_text(
    workspace_path: &str,
    conversation_id: &str,
    state: &mut PipelineState,
) {
    let merged_file = conversation_dir(workspace_path, conversation_id)
        .join("plan_merged")
        .join("plan_merged.md");

    for stage in &mut state.stages {
        if let Some(path) = artifact_path_for_stage(workspace_path, conversation_id, stage) {
            hydrate_from_file(stage, &path);
        }
    }

    let has_merge_stage = state
        .stages
        .iter()
        .any(|stage| stage.stage_name == "Plan Merge");
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
                score_id: None,
                provider_session_ref: None,
                user_prompt: None,
            });
        }
    }
}
