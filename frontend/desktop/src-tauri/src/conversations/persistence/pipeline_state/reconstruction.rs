use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};

use super::super::paths::conversation_dir;
use super::save_pipeline_state;

pub(super) fn reconstruct_pipeline_from_artifacts(
    workspace_path: &str,
    conversation_id: &str,
) -> Option<PipelineState> {
    let conversation_root = conversation_dir(workspace_path, conversation_id);
    let prompt_path = conversation_root.join("prompt").join("prompt.md");
    let plan_dir = conversation_root.join("plan");
    let user_prompt = std::fs::read_to_string(&prompt_path).ok()?;

    let mut stages: Vec<PipelineStageRecord> = Vec::new();
    if plan_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&plan_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(rest) = name_str.strip_prefix("Plan-") {
                    if let Some(num_str) = rest.strip_suffix(".md") {
                        if let Ok(planner_index) = num_str.parse::<usize>() {
                            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                            stages.push(PipelineStageRecord {
                                stage_index: planner_index - 1,
                                stage_name: format!("Planner {planner_index}"),
                                agent_label: String::new(),
                                status: ConversationStatus::Completed,
                                text,
                                started_at: None,
                                finished_at: None,
                                score_id: None,
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
            score_id: None,
            provider_session_ref: None,
        });
    }
    stages.sort_by_key(|stage| stage.stage_index);

    append_stage_from_file(
        &mut stages,
        &conversation_root.join("plan_merged").join("plan_merged.md"),
        "Plan Merge",
    );
    append_stage_from_file(
        &mut stages,
        &conversation_root.join("coder").join("coder_done.md"),
        "Coder",
    );

    let review_dir = conversation_root.join("review");
    if review_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&review_dir) {
            let mut review_stages: Vec<PipelineStageRecord> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(rest) = name_str.strip_prefix("Review-") {
                    if let Some(num_str) = rest.strip_suffix(".md") {
                        if let Ok(reviewer_index) = num_str.parse::<usize>() {
                            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                            review_stages.push(PipelineStageRecord {
                                stage_index: stages.len() + reviewer_index - 1,
                                stage_name: format!("Reviewer {reviewer_index}"),
                                agent_label: String::new(),
                                status: ConversationStatus::Completed,
                                text,
                                started_at: None,
                                finished_at: None,
                                score_id: None,
                                provider_session_ref: None,
                            });
                        }
                    }
                }
            }

            review_stages.sort_by_key(|stage| stage.stage_index);
            for (index, review_stage) in review_stages.iter_mut().enumerate() {
                review_stage.stage_index = stages.len() + index;
            }
            stages.extend(review_stages);
        }
    }

    append_stage_from_file(
        &mut stages,
        &conversation_root.join("review_merged").join("review_merged.md"),
        "Review Merge",
    );
    append_stage_from_file(
        &mut stages,
        &conversation_root.join("code_fixer").join("code_fixer_done.md"),
        "Code Fixer",
    );

    let state = PipelineState {
        user_prompt,
        pipeline_mode: "code".to_string(),
        stages,
    };

    if let Err(error) = save_pipeline_state(workspace_path, conversation_id, &state) {
        eprintln!("[pipeline] Failed to persist reconstructed pipeline state: {error}");
    }

    Some(state)
}

fn append_stage_from_file(
    stages: &mut Vec<PipelineStageRecord>,
    path: &std::path::Path,
    stage_name: &str,
) {
    if !path.exists() {
        return;
    }

    let text = std::fs::read_to_string(path).unwrap_or_default();
    stages.push(PipelineStageRecord {
        stage_index: stages.len(),
        stage_name: stage_name.to_string(),
        agent_label: String::new(),
        status: ConversationStatus::Completed,
        text,
        started_at: None,
        finished_at: None,
        score_id: None,
        provider_session_ref: None,
    });
}
