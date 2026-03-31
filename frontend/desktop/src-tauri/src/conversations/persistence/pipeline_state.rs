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
            stage.score_id.clone_from(&record.score_id);
            stage
                .provider_session_ref
                .clone_from(&record.provider_session_ref);
            stage.started_at.clone_from(&record.started_at);
            stage.finished_at.clone_from(&record.finished_at);
        }

        save_pipeline_state(workspace_path, conversation_id, &state)
    })
}

/// Try to read a file and, if it exists, set the stage text and fix stale
/// Running status. Also fills in `finished_at` when it was never persisted
/// (e.g. after a crash), using the file's modification time as a proxy.
fn hydrate_from_file(stage: &mut PipelineStageRecord, path: &std::path::Path) {
    if let Ok(contents) = std::fs::read_to_string(path) {
        stage.text = contents;
        if stage.status == ConversationStatus::Running {
            stage.status = ConversationStatus::Completed;
        }
        if stage.finished_at.is_none() {
            stage.finished_at = file_modified_rfc3339(path)
                .or_else(|| stage.started_at.clone());
        }
    }
}

/// Return the file's last-modified time as an RFC 3339 string, or None on error.
fn file_modified_rfc3339(path: &std::path::Path) -> Option<String> {
    let mtime = std::fs::metadata(path).ok()?.modified().ok()?;
    let dt: chrono::DateTime<chrono::Utc> = mtime.into();
    Some(dt.to_rfc3339())
}

/// Fills in the `text` field of each stage from the corresponding artifact
/// file on disk. Also corrects stale "Running" status when the file already
/// exists.
fn hydrate_stage_text(workspace_path: &str, conversation_id: &str, state: &mut PipelineState) {
    let conv_dir = conversation_dir(workspace_path, conversation_id);
    let plan_dir = conv_dir.join("plan");
    let merged_file = conv_dir.join("plan_merged").join("plan_merged.md");
    let coder_file = conv_dir.join("coder").join("coder_done.md");
    let review_dir = conv_dir.join("review");
    let review_merged_file = conv_dir.join("review_merged").join("review_merged.md");
    let code_fixer_file = conv_dir.join("code_fixer").join("code_fixer_done.md");

    for stage in &mut state.stages {
        if stage.stage_name.starts_with("Planner") {
            let plan_file = plan_dir.join(format!("Plan-{}.md", stage.stage_index + 1));
            hydrate_from_file(stage, &plan_file);
        } else if stage.stage_name == "Plan Merge" {
            hydrate_from_file(stage, &merged_file);
        } else if stage.stage_name == "Coder" {
            hydrate_from_file(stage, &coder_file);
        } else if stage.stage_name.starts_with("Reviewer") {
            // Extract reviewer number from stage name (e.g. "Reviewer 2" → 2).
            if let Some(num_str) = stage.stage_name.strip_prefix("Reviewer ") {
                if let Ok(n) = num_str.parse::<usize>() {
                    let review_file = review_dir.join(format!("Review-{n}.md"));
                    hydrate_from_file(stage, &review_file);
                }
            }
        } else if stage.stage_name == "Review Merge" {
            hydrate_from_file(stage, &review_merged_file);
        } else if stage.stage_name == "Code Fixer" {
            hydrate_from_file(stage, &code_fixer_file);
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
                score_id: None,
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
            score_id: None,
            provider_session_ref: None,
        });
    }

    // Coder completion marker.
    let coder_file = conv_dir.join("coder").join("coder_done.md");
    if coder_file.exists() {
        let text = std::fs::read_to_string(&coder_file).unwrap_or_default();
        stages.push(PipelineStageRecord {
            stage_index: stages.len(),
            stage_name: "Coder".to_string(),
            agent_label: String::new(),
            status: ConversationStatus::Completed,
            text,
            started_at: None,
            finished_at: None,
            score_id: None,
            provider_session_ref: None,
        });
    }

    // Individual reviewer outputs.
    let review_dir = conv_dir.join("review");
    if review_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&review_dir) {
            let mut review_stages: Vec<PipelineStageRecord> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(rest) = name_str.strip_prefix("Review-") {
                    if let Some(num_str) = rest.strip_suffix(".md") {
                        if let Ok(n) = num_str.parse::<usize>() {
                            let text =
                                std::fs::read_to_string(entry.path()).unwrap_or_default();
                            review_stages.push(PipelineStageRecord {
                                stage_index: stages.len() + n - 1,
                                stage_name: format!("Reviewer {n}"),
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
            review_stages.sort_by_key(|s| s.stage_index);
            // Fix indices sequentially after existing stages.
            for (i, rs) in review_stages.iter_mut().enumerate() {
                rs.stage_index = stages.len() + i;
            }
            stages.extend(review_stages);
        }
    }

    // Review merge output.
    let review_merged_file = conv_dir.join("review_merged").join("review_merged.md");
    if review_merged_file.exists() {
        let text = std::fs::read_to_string(&review_merged_file).unwrap_or_default();
        stages.push(PipelineStageRecord {
            stage_index: stages.len(),
            stage_name: "Review Merge".to_string(),
            agent_label: String::new(),
            status: ConversationStatus::Completed,
            text,
            started_at: None,
            finished_at: None,
            score_id: None,
            provider_session_ref: None,
        });
    }

    // Code Fixer completion marker.
    let code_fixer_file = conv_dir.join("code_fixer").join("code_fixer_done.md");
    if code_fixer_file.exists() {
        let text = std::fs::read_to_string(&code_fixer_file).unwrap_or_default();
        stages.push(PipelineStageRecord {
            stage_index: stages.len(),
            stage_name: "Code Fixer".to_string(),
            agent_label: String::new(),
            status: ConversationStatus::Completed,
            text,
            started_at: None,
            finished_at: None,
            score_id: None,
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
