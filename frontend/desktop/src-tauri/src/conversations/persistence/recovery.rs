use crate::models::{
    AgentSelection, ConversationMessageRole, ConversationStatus, ConversationSummary, PipelineState,
};
use crate::storage::{atomic_write, now_rfc3339};

use super::io::{read_messages_unlocked, read_summary_unlocked, write_summary_unlocked};
use super::paths::{
    conversation_backup_file_path, conversation_dir, conversation_file_path, orchestrator_output_path,
    pipeline_file_path, plan_dir_path, prompt_file_path, RECOVERED_SUMMARY_ERROR, STALE_RUNNING_ERROR,
};
use super::pipeline_state::load_pipeline_state;
use super::registries::is_running_conversation_tracked;

pub(super) fn normalise_title(prompt: &str) -> String {
    let trimmed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return "New conversation".to_string();
    }

    let mut title = String::new();
    let mut count = 0usize;
    for ch in trimmed.chars() {
        if count >= 48 {
            break;
        }
        title.push(ch);
        count += 1;
    }
    if trimmed.chars().count() > 48 {
        title.push_str("...");
    }
    title
}

pub(super) fn parse_agent_label(label: &str) -> Option<AgentSelection> {
    let (provider, model) = label.split_once(" / ")?;
    let provider = provider.trim();
    let model = model.trim();
    if provider.is_empty() || model.is_empty() {
        return None;
    }

    Some(AgentSelection {
        provider: provider.to_string(),
        model: model.to_string(),
    })
}

/// Try to read the orchestrator output and extract the summary title.
fn try_read_orchestrator_title(workspace_path: &str, conversation_id: &str) -> Option<String> {
    let path = orchestrator_output_path(workspace_path, conversation_id);
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn recover_title_unlocked(
    workspace_path: &str,
    conversation_id: &str,
    messages: &[crate::models::ConversationMessage],
) -> String {
    // First try the orchestrator-generated summary (most accurate).
    if let Some(title) = try_read_orchestrator_title(workspace_path, conversation_id) {
        return title;
    }

    if let Some(title) = messages
        .iter()
        .find(|message| message.role == ConversationMessageRole::User)
        .map(|message| normalise_title(&message.content))
    {
        return title;
    }

    if let Ok(prompt) = std::fs::read_to_string(prompt_file_path(workspace_path, conversation_id)) {
        let title = normalise_title(&prompt);
        if !title.trim().is_empty() {
            return title;
        }
    }

    "Recovered conversation".to_string()
}

fn recover_status_from_pipeline_state(state: &PipelineState) -> ConversationStatus {
    if state
        .stages
        .iter()
        .any(|stage| stage.status == ConversationStatus::Running)
    {
        return ConversationStatus::Failed;
    }

    if state
        .stages
        .iter()
        .any(|stage| stage.status == ConversationStatus::Failed)
    {
        return ConversationStatus::Failed;
    }

    if !state.stages.is_empty()
        && state
            .stages
            .iter()
            .all(|stage| stage.status == ConversationStatus::Completed)
    {
        return ConversationStatus::Completed;
    }

    ConversationStatus::Idle
}

fn recover_agent_selection(pipeline_state: Option<&PipelineState>) -> AgentSelection {
    pipeline_state
        .and_then(|state| {
            state
                .stages
                .iter()
                .find_map(|stage| parse_agent_label(&stage.agent_label))
        })
        .unwrap_or_else(|| AgentSelection {
            provider: "unknown".to_string(),
            model: "unknown".to_string(),
        })
}

fn recover_summary_unlocked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Option<ConversationSummary>, String> {
    let messages = read_messages_unlocked(workspace_path, conversation_id)?;
    let pipeline_state = load_pipeline_state(workspace_path, conversation_id)
        .ok()
        .flatten();
    let has_artifacts = !messages.is_empty()
        || pipeline_state.is_some()
        || prompt_file_path(workspace_path, conversation_id).exists()
        || plan_dir_path(workspace_path, conversation_id).is_dir();

    if !has_artifacts {
        let dir = conversation_dir(workspace_path, conversation_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|error| {
                format!(
                    "Failed to delete orphaned conversation {}: {error}",
                    dir.display()
                )
            })?;
        }
        return Ok(None);
    }

    let created_at = messages
        .first()
        .map(|message| message.created_at.clone())
        .unwrap_or_else(now_rfc3339);
    let updated_at = messages
        .last()
        .map(|message| message.created_at.clone())
        .unwrap_or_else(now_rfc3339);
    let status = pipeline_state
        .as_ref()
        .map(recover_status_from_pipeline_state)
        .unwrap_or_else(|| {
            if messages.is_empty() {
                ConversationStatus::Failed
            } else if matches!(
                messages.last().map(|message| &message.role),
                Some(ConversationMessageRole::Assistant)
            ) {
                ConversationStatus::Completed
            } else {
                ConversationStatus::Failed
            }
        });

    let summary = ConversationSummary {
        id: conversation_id.to_string(),
        title: recover_title_unlocked(workspace_path, conversation_id, &messages),
        workspace_path: workspace_path.to_string(),
        agent: recover_agent_selection(pipeline_state.as_ref()),
        status: status.clone(),
        created_at,
        updated_at,
        message_count: messages.len(),
        last_provider_session_ref: None,
        active_score_id: None,
        error: (status == ConversationStatus::Failed).then(|| RECOVERED_SUMMARY_ERROR.to_string()),
        archived_at: None,
        pinned_at: None,
    };
    write_summary_unlocked(&summary)?;
    Ok(Some(summary))
}

pub(super) fn load_summary_with_recovery_unlocked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Option<(ConversationSummary, bool)>, String> {
    match read_summary_unlocked(workspace_path, conversation_id) {
        Ok(summary) => Ok(Some((summary, false))),
        Err(error) => {
            let summary_path = conversation_file_path(workspace_path, conversation_id);
            if !summary_path.exists() {
                let backup_path = conversation_backup_file_path(workspace_path, conversation_id);
                if backup_path.exists() {
                    std::fs::rename(&backup_path, &summary_path).map_err(|restore_error| {
                        format!(
                            "Failed to restore backup {} to {}: {restore_error}",
                            backup_path.display(),
                            summary_path.display()
                        )
                    })?;
                    let restored = read_summary_unlocked(workspace_path, conversation_id)?;
                    return Ok(Some((restored, true)));
                }

                return recover_summary_unlocked(workspace_path, conversation_id)
                    .map(|summary| summary.map(|value| (value, true)));
            }

            Err(error)
        }
    }
}

pub(super) fn reconcile_stale_running_unlocked(
    summary: &mut ConversationSummary,
) -> Result<(), String> {
    if summary.status != ConversationStatus::Running {
        return Ok(());
    }
    if is_running_conversation_tracked(&summary.workspace_path, &summary.id)? {
        return Ok(());
    }

    summary.status = ConversationStatus::Failed;
    summary.active_score_id = None;
    summary.error = Some(STALE_RUNNING_ERROR.to_string());
    summary.updated_at = now_rfc3339();
    write_summary_unlocked(summary)?;

    reconcile_stale_pipeline_stages(&summary.workspace_path, &summary.id);
    Ok(())
}

fn reconcile_stale_pipeline_stages(workspace_path: &str, conversation_id: &str) {
    let path = pipeline_file_path(workspace_path, conversation_id);
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let mut state: PipelineState = match serde_json::from_str(&data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let plan_dir = conversation_dir(workspace_path, conversation_id).join("plan");
    let mut changed = false;

    for stage in &mut state.stages {
        if stage.status != ConversationStatus::Running {
            continue;
        }
        let plan_file = plan_dir.join(format!("Plan-{}.md", stage.stage_index + 1));
        if plan_file.exists() {
            stage.status = ConversationStatus::Completed;
            stage.finished_at = Some(now_rfc3339());
        } else {
            stage.status = ConversationStatus::Failed;
            stage.finished_at = Some(now_rfc3339());
        }
        changed = true;
    }

    if changed {
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = atomic_write(&path, &json);
        }
    }
}
