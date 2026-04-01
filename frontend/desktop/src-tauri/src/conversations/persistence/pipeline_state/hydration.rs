use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};

use super::super::paths::conversation_dir;

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
    let conversation_root = conversation_dir(workspace_path, conversation_id);
    let plan_dir = conversation_root.join("plan");
    let merged_file = conversation_root.join("plan_merged").join("plan_merged.md");
    let coder_file = conversation_root.join("coder").join("coder_done.md");
    let review_dir = conversation_root.join("review");
    let review_merged_file = conversation_root.join("review_merged").join("review_merged.md");
    let code_fixer_file = conversation_root.join("code_fixer").join("code_fixer_done.md");

    for stage in &mut state.stages {
        if stage.stage_name.starts_with("Planner") {
            let plan_file = plan_dir.join(format!("Plan-{}.md", stage.stage_index + 1));
            hydrate_from_file(stage, &plan_file);
        } else if stage.stage_name == "Plan Merge" {
            hydrate_from_file(stage, &merged_file);
        } else if stage.stage_name == "Coder" {
            hydrate_from_file(stage, &coder_file);
        } else if stage.stage_name.starts_with("Reviewer") {
            if let Some(reviewer_number) = stage.stage_name.strip_prefix("Reviewer ") {
                if let Ok(reviewer_index) = reviewer_number.parse::<usize>() {
                    let review_file = review_dir.join(format!("Review-{reviewer_index}.md"));
                    hydrate_from_file(stage, &review_file);
                }
            }
        } else if stage.stage_name == "Review Merge" {
            hydrate_from_file(stage, &review_merged_file);
        } else if stage.stage_name == "Code Fixer" {
            hydrate_from_file(stage, &code_fixer_file);
        }
    }

    let has_merge_stage = state.stages.iter().any(|stage| stage.stage_name == "Plan Merge");
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
            });
        }
    }
}
