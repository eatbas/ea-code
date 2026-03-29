use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::models::{
    AgentSelection, ConversationDetail, ConversationMessage, ConversationMessageRole,
    ConversationStatus, ConversationSummary, PipelineStageRecord, PipelineState,
};
use crate::storage::{atomic_write, now_rfc3339, with_conversations_lock};

const CONVERSATIONS_DIR: &str = ".ea-code/conversations";
const CONVERSATION_FILE: &str = "conversation.json";
const MESSAGES_FILE: &str = "messages.jsonl";
const PIPELINE_FILE: &str = "pipeline.json";
const STALE_RUNNING_ERROR: &str = "ea-code closed while this task was running";
const RECOVERED_SUMMARY_ERROR: &str = "Recovered conversation metadata after an incomplete write";

fn running_conversations() -> &'static Mutex<HashSet<String>> {
    static ACTIVE_RUNNING_CONVERSATIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ACTIVE_RUNNING_CONVERSATIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn running_conversation_key(workspace_path: &str, conversation_id: &str) -> String {
    format!("{workspace_path}::{conversation_id}")
}

fn is_running_conversation_tracked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<bool, String> {
    running_conversations()
        .lock()
        .map(|tracked| tracked.contains(&running_conversation_key(workspace_path, conversation_id)))
        .map_err(|error| format!("Failed to inspect running conversations: {error}"))
}

pub struct RunningConversationGuard {
    key: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ConversationCleanupStats {
    pub recovered: usize,
    pub removed: usize,
}

impl Drop for RunningConversationGuard {
    fn drop(&mut self) {
        if let Ok(mut tracked) = running_conversations().lock() {
            tracked.remove(&self.key);
        }
    }
}

pub fn track_running_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<RunningConversationGuard, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    running_conversations()
        .lock()
        .map_err(|error| format!("Failed to track running conversation: {error}"))?
        .insert(key.clone());
    Ok(RunningConversationGuard { key })
}

fn abort_flags() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static FLAGS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_abort_flag(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Arc<AtomicBool>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let flag = Arc::new(AtomicBool::new(false));
    abort_flags()
        .lock()
        .map_err(|error| format!("Failed to register abort flag: {error}"))?
        .insert(key, flag.clone());
    Ok(flag)
}

pub fn trigger_abort(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    if let Some(flag) = abort_flags()
        .lock()
        .map_err(|error| format!("Failed to trigger abort: {error}"))?
        .get(&key)
    {
        flag.store(true, Ordering::Release);
    }
    Ok(())
}

pub fn remove_abort_flag(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    abort_flags()
        .lock()
        .map_err(|error| format!("Failed to remove abort flag: {error}"))?
        .remove(&key);
    Ok(())
}

// ---------------------------------------------------------------------------
// Pipeline job-ID registry — tracks live hive-api job IDs so stop_pipeline
// can cancel them on the server side.
// ---------------------------------------------------------------------------

fn pipeline_jobs() -> &'static Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<Option<String>>>>>> {
    static JOBS: OnceLock<Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<Option<String>>>>>>> =
        OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Pre-allocate empty job-ID slots for each planner. Call *before* spawning
/// the pipeline task so that `get_pipeline_job_ids` can read them even if
/// `stop_pipeline` is called immediately.
pub fn register_pipeline_job_slots(
    workspace_path: &str,
    conversation_id: &str,
    count: usize,
) -> Result<Vec<Arc<std::sync::Mutex<Option<String>>>>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let slots: Vec<_> = (0..count)
        .map(|_| Arc::new(std::sync::Mutex::new(None)))
        .collect();
    pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to register pipeline job slots: {e}"))?
        .insert(key, slots.clone());
    Ok(slots)
}

/// Return every non-empty job ID currently registered for this pipeline.
pub fn get_pipeline_job_ids(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<String>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let guard = pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to read pipeline job slots: {e}"))?;
    let Some(slots) = guard.get(&key) else {
        return Ok(Vec::new());
    };
    Ok(slots
        .iter()
        .filter_map(|slot| slot.lock().ok().and_then(|g| g.clone()))
        .collect())
}

/// Remove all job-ID slots for a finished pipeline.
pub fn remove_pipeline_job_slots(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to remove pipeline job slots: {e}"))?
        .remove(&key);
    Ok(())
}

// ---------------------------------------------------------------------------
// Pipeline stage output buffers — accumulates SSE output text on the Rust
// side so it survives frontend navigation (React state is ephemeral).
// ---------------------------------------------------------------------------

fn stage_buffers() -> &'static Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<String>>>>> {
    static BUFS: OnceLock<Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<String>>>>>> =
        OnceLock::new();
    BUFS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Pre-allocate one text buffer per planner. Each planner appends its SSE
/// output here so `get_pipeline_state` can return accumulated text even
/// after the frontend navigated away and back.
pub fn register_pipeline_stage_buffers(
    workspace_path: &str,
    conversation_id: &str,
    count: usize,
) -> Result<Vec<Arc<std::sync::Mutex<String>>>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let buffers: Vec<_> = (0..count)
        .map(|_| Arc::new(std::sync::Mutex::new(String::new())))
        .collect();
    stage_buffers()
        .lock()
        .map_err(|e| format!("Failed to register stage buffers: {e}"))?
        .insert(key, buffers.clone());
    Ok(buffers)
}

/// Read accumulated output text from every registered stage buffer.
pub fn get_pipeline_stage_texts(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<String>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let guard = stage_buffers()
        .lock()
        .map_err(|e| format!("Failed to read stage buffers: {e}"))?;
    let Some(buffers) = guard.get(&key) else {
        return Ok(Vec::new());
    };
    Ok(buffers
        .iter()
        .map(|buf| buf.lock().map(|g| g.clone()).unwrap_or_default())
        .collect())
}

/// Remove all stage buffers for a finished pipeline.
pub fn remove_pipeline_stage_buffers(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    stage_buffers()
        .lock()
        .map_err(|e| format!("Failed to remove stage buffers: {e}"))?
        .remove(&key);
    Ok(())
}

fn conversations_dir(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path).join(CONVERSATIONS_DIR)
}

fn conversation_dir(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversations_dir(workspace_path).join(conversation_id)
}

fn conversation_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(CONVERSATION_FILE)
}

fn conversation_backup_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    let path = conversation_file_path(workspace_path, conversation_id);
    PathBuf::from(format!("{}.bak", path.to_string_lossy()))
}

fn messages_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(MESSAGES_FILE)
}

fn prompt_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id)
        .join("prompt")
        .join("prompt.md")
}

fn plan_dir_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join("plan")
}

fn read_summary_unlocked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    let path = conversation_file_path(workspace_path, conversation_id);
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read conversation {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Failed to parse conversation {}: {error}", path.display()))
}

fn write_summary_unlocked(summary: &ConversationSummary) -> Result<(), String> {
    let path = conversation_file_path(&summary.workspace_path, &summary.id);
    let json = serde_json::to_string_pretty(summary).map_err(|error| {
        format!(
            "Failed to serialise conversation {}: {error}",
            path.display()
        )
    })?;
    atomic_write(&path, &json)
}

fn read_messages_unlocked(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<ConversationMessage>, String> {
    let path = messages_file_path(workspace_path, conversation_id);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read messages {}: {error}", path.display()))?;

    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<ConversationMessage>(line).map_err(|error| {
                format!(
                    "Failed to parse message entry in {}: {error}",
                    path.display()
                )
            })
        })
        .collect()
}

fn write_messages_unlocked(
    workspace_path: &str,
    conversation_id: &str,
    messages: &[ConversationMessage],
) -> Result<(), String> {
    let path = messages_file_path(workspace_path, conversation_id);
    let mut contents = String::new();
    for message in messages {
        let line = serde_json::to_string(message).map_err(|error| {
            format!(
                "Failed to serialise message for {}: {error}",
                path.display()
            )
        })?;
        contents.push_str(&line);
        contents.push('\n');
    }
    atomic_write(&path, &contents)
}

fn normalise_title(prompt: &str) -> String {
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

fn parse_agent_label(label: &str) -> Option<AgentSelection> {
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

fn recover_title_unlocked(
    workspace_path: &str,
    conversation_id: &str,
    messages: &[ConversationMessage],
) -> String {
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
        active_job_id: None,
        error: (status == ConversationStatus::Failed).then(|| RECOVERED_SUMMARY_ERROR.to_string()),
        archived_at: None,
        pinned_at: None,
    };
    write_summary_unlocked(&summary)?;
    Ok(Some(summary))
}

fn load_summary_with_recovery_unlocked(
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

fn reconcile_stale_running_unlocked(summary: &mut ConversationSummary) -> Result<(), String> {
    if summary.status != ConversationStatus::Running {
        return Ok(());
    }
    if is_running_conversation_tracked(&summary.workspace_path, &summary.id)? {
        return Ok(());
    }

    summary.status = ConversationStatus::Failed;
    summary.active_job_id = None;
    summary.error = Some(STALE_RUNNING_ERROR.to_string());
    summary.updated_at = now_rfc3339();
    write_summary_unlocked(summary)?;

    // Reconcile stale pipeline stages — check plan files to determine real status.
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
        // Check if the planner's plan file exists.
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

fn build_detail_unlocked(summary: ConversationSummary) -> Result<ConversationDetail, String> {
    let messages = read_messages_unlocked(&summary.workspace_path, &summary.id)?;
    Ok(ConversationDetail { summary, messages })
}

pub fn create_conversation(
    workspace_path: &str,
    agent: AgentSelection,
    initial_prompt: Option<&str>,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let now = now_rfc3339();
        let summary = ConversationSummary {
            id: uuid::Uuid::new_v4().to_string(),
            title: initial_prompt
                .map(normalise_title)
                .unwrap_or_else(|| "New conversation".to_string()),
            workspace_path: workspace_path.to_string(),
            agent,
            status: ConversationStatus::Idle,
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
            last_provider_session_ref: None,
            active_job_id: None,
            error: None,
            archived_at: None,
            pinned_at: None,
        };
        write_summary_unlocked(&summary)?;
        write_messages_unlocked(workspace_path, &summary.id, &[])?;
        build_detail_unlocked(summary)
    })
}

pub fn list_conversations(
    workspace_path: &str,
    include_archived: bool,
) -> Result<Vec<ConversationSummary>, String> {
    with_conversations_lock(|| {
        let dir = conversations_dir(workspace_path);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut summaries: Vec<ConversationSummary> = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|error| {
            format!(
                "Failed to read conversations directory {}: {error}",
                dir.display()
            )
        })? {
            let entry =
                entry.map_err(|error| format!("Failed to read conversation entry: {error}"))?;
            if !entry.path().is_dir() {
                continue;
            }

            let conversation_id = match entry.file_name().to_str() {
                Some(value) => value.to_string(),
                None => continue,
            };
            let (mut summary, recovered) =
                match load_summary_with_recovery_unlocked(workspace_path, &conversation_id) {
                    Ok(Some(summary)) => summary,
                    Ok(None) => {
                        eprintln!(
                            "[conversations] Removed orphaned conversation {conversation_id}"
                        );
                        continue;
                    }
                    Err(e) => {
                        eprintln!("[conversations] Skipping {conversation_id}: {e}");
                        continue;
                    }
                };
            if recovered {
                eprintln!("[conversations] Recovered conversation {conversation_id}");
            }
            if let Err(e) = reconcile_stale_running_unlocked(&mut summary) {
                eprintln!("[conversations] Reconcile failed for {conversation_id}: {e}");
            }
            if include_archived || summary.archived_at.is_none() {
                summaries.push(summary);
            }
        }

        summaries.sort_by(|left, right| {
            right
                .pinned_at
                .is_some()
                .cmp(&left.pinned_at.is_some())
                .then_with(|| right.updated_at.cmp(&left.updated_at))
        });
        Ok(summaries)
    })
}

pub fn cleanup_orphaned_conversations(
    workspace_path: &str,
) -> Result<ConversationCleanupStats, String> {
    with_conversations_lock(|| {
        let dir = conversations_dir(workspace_path);
        if !dir.exists() {
            return Ok(ConversationCleanupStats::default());
        }

        let mut stats = ConversationCleanupStats::default();
        for entry in std::fs::read_dir(&dir).map_err(|error| {
            format!(
                "Failed to read conversations directory {}: {error}",
                dir.display()
            )
        })? {
            let entry =
                entry.map_err(|error| format!("Failed to read conversation entry: {error}"))?;
            if !entry.path().is_dir() {
                continue;
            }

            let conversation_id = match entry.file_name().to_str() {
                Some(value) => value.to_string(),
                None => continue,
            };

            match load_summary_with_recovery_unlocked(workspace_path, &conversation_id) {
                Ok(Some((_summary, true))) => {
                    stats.recovered += 1;
                }
                Ok(Some((_summary, false))) => {}
                Ok(None) => {
                    stats.removed += 1;
                }
                Err(error) => {
                    eprintln!("[conversations] Cleanup skipped {conversation_id}: {error}");
                }
            }
        }

        Ok(stats)
    })
}

pub fn get_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        reconcile_stale_running_unlocked(&mut summary)?;
        build_detail_unlocked(summary)
    })
}

pub fn mark_turn_running(
    workspace_path: &str,
    conversation_id: &str,
    prompt: &str,
) -> Result<ConversationDetail, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        reconcile_stale_running_unlocked(&mut summary)?;
        if summary.status == ConversationStatus::Running {
            return Err("This conversation is already running".to_string());
        }

        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;
        let user_message = ConversationMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: ConversationMessageRole::User,
            content: prompt.to_string(),
            created_at: now_rfc3339(),
        };
        messages.push(user_message);
        summary.message_count = messages.len();
        if summary.message_count == 1 {
            summary.title = normalise_title(prompt);
        }
        summary.status = ConversationStatus::Running;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        summary.error = None;

        write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        write_summary_unlocked(&summary)?;

        Ok(ConversationDetail { summary, messages })
    })
}

pub fn set_active_job_id(
    workspace_path: &str,
    conversation_id: &str,
    job_id: Option<String>,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.active_job_id = job_id;
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn set_provider_session_ref(
    workspace_path: &str,
    conversation_id: &str,
    provider_session_ref: String,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.last_provider_session_ref = Some(provider_session_ref);
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn set_status(
    workspace_path: &str,
    conversation_id: &str,
    status: ConversationStatus,
    error: Option<String>,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.status = status;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        summary.error = error;
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
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

pub fn finish_turn(
    workspace_path: &str,
    conversation_id: &str,
    status: ConversationStatus,
    assistant_text: Option<String>,
    provider_session_ref: Option<String>,
    error: Option<String>,
) -> Result<(ConversationSummary, Option<ConversationMessage>), String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        let mut messages = read_messages_unlocked(workspace_path, conversation_id)?;

        let assistant_message =
            assistant_text
                .filter(|text| !text.trim().is_empty())
                .map(|content| ConversationMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: ConversationMessageRole::Assistant,
                    content,
                    created_at: now_rfc3339(),
                });

        if let Some(message) = &assistant_message {
            messages.push(message.clone());
            write_messages_unlocked(workspace_path, conversation_id, &messages)?;
        }

        summary.message_count = messages.len();
        summary.status = status;
        summary.updated_at = now_rfc3339();
        summary.active_job_id = None;
        if provider_session_ref.is_some() {
            summary.last_provider_session_ref = provider_session_ref;
        }
        summary.error = error;
        write_summary_unlocked(&summary)?;

        Ok((summary, assistant_message))
    })
}

pub fn delete_conversation(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    with_conversations_lock(|| {
        let summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.status == ConversationStatus::Running {
            return Err("Cannot delete a running conversation".to_string());
        }

        let dir = conversation_dir(workspace_path, conversation_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|error| {
                format!("Failed to delete conversation {}: {error}", dir.display())
            })?;
        }
        Ok(())
    })
}

pub fn rename_conversation(
    workspace_path: &str,
    conversation_id: &str,
    title: &str,
) -> Result<ConversationSummary, String> {
    let trimmed = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return Err("Conversation title must not be empty".to_string());
    }

    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.title = trimmed;
        summary.updated_at = now_rfc3339();
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

pub fn archive_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.status == ConversationStatus::Running {
            return Err("Cannot archive a running conversation".to_string());
        }

        if summary.archived_at.is_none() {
            summary.archived_at = Some(now_rfc3339());
            summary.updated_at = now_rfc3339();
            write_summary_unlocked(&summary)?;
        }

        Ok(summary)
    })
}

pub fn unarchive_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        if summary.archived_at.is_some() {
            summary.archived_at = None;
            summary.updated_at = now_rfc3339();
            write_summary_unlocked(&summary)?;
        }

        Ok(summary)
    })
}

pub fn set_conversation_pinned(
    workspace_path: &str,
    conversation_id: &str,
    pinned: bool,
) -> Result<ConversationSummary, String> {
    with_conversations_lock(|| {
        let mut summary = read_summary_unlocked(workspace_path, conversation_id)?;
        summary.pinned_at = if pinned { Some(now_rfc3339()) } else { None };
        write_summary_unlocked(&summary)?;
        Ok(summary)
    })
}

// ── Pipeline state persistence ───────────────────────────────────────

fn pipeline_file_path(workspace_path: &str, conversation_id: &str) -> PathBuf {
    conversation_dir(workspace_path, conversation_id).join(PIPELINE_FILE)
}

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

/// Atomically update a single stage inside pipeline.json.  Called when an
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
            stage.provider_session_ref.clone_from(&record.provider_session_ref);
            stage.started_at.clone_from(&record.started_at);
            stage.finished_at.clone_from(&record.finished_at);
        }

        save_pipeline_state(workspace_path, conversation_id, &state)
    })
}

/// Fills in the `text` field of each stage from the corresponding plan file
/// on disk. Also corrects stale "Running" status when the plan file already
/// exists — this handles the window between a planner finishing and the next
/// `save_pipeline_state` call.
fn hydrate_stage_text(workspace_path: &str, conversation_id: &str, state: &mut PipelineState) {
    let plan_dir = conversation_dir(workspace_path, conversation_id).join("plan");
    for stage in &mut state.stages {
        let plan_file = plan_dir.join(format!("Plan-{}.md", stage.stage_index + 1));
        if let Ok(contents) = std::fs::read_to_string(&plan_file) {
            // The plan file is the authoritative deliverable — always prefer
            // it over SSE output text that may be stored in the record.
            stage.text = contents;
            // A plan file on disk proves the planner finished its work, even
            // if pipeline.json still says "Running" (stale state).
            if stage.status == ConversationStatus::Running {
                stage.status = ConversationStatus::Completed;
            }
        }
    }
}

/// Reconstructs a PipelineState from disk artifacts when pipeline.json is missing
/// (e.g. crash before first save in older versions).
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
                            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        archive_conversation, create_conversation, delete_conversation, finish_turn,
        get_conversation, list_conversations, mark_turn_running, rename_conversation,
        set_conversation_pinned, track_running_conversation, unarchive_conversation,
    };
    use crate::models::{AgentSelection, ConversationStatus};

    struct TestWorkspace {
        path: PathBuf,
    }

    impl TestWorkspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("ea-code-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("temporary workspace should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn create_and_list_conversations() {
        let workspace = TestWorkspace::new();
        let first = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Investigate the build failure"),
        )
        .expect("conversation should be created");

        let listed = list_conversations(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            false,
        )
        .expect("conversations should list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, first.summary.id);
        assert_eq!(listed[0].title, "Investigate the build failure");
    }

    #[test]
    fn turn_start_and_finish_persist_messages() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "claude".to_string(),
                model: "sonnet".to_string(),
            },
            None,
        )
        .expect("conversation should be created");

        let running = mark_turn_running(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
            "Explain the app structure",
        )
        .expect("turn should start");
        assert_eq!(running.summary.status, ConversationStatus::Running);
        assert_eq!(running.messages.len(), 1);

        finish_turn(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
            ConversationStatus::Completed,
            Some("The app has a Tauri backend and React frontend.".to_string()),
            Some("session-123".to_string()),
            None,
        )
        .expect("turn should finish");

        let loaded = get_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should load");

        assert_eq!(loaded.summary.status, ConversationStatus::Completed);
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(
            loaded.summary.last_provider_session_ref.as_deref(),
            Some("session-123")
        );
    }

    #[test]
    fn stale_running_conversations_reconcile_on_load() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            None,
        )
        .expect("conversation should be created");

        mark_turn_running(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
            "Continue the last task",
        )
        .expect("turn should start");

        let loaded = get_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should load");

        assert_eq!(loaded.summary.status, ConversationStatus::Failed);
        assert_eq!(
            loaded.summary.error.as_deref(),
            Some("ea-code closed while this task was running")
        );
    }

    #[test]
    fn tracked_running_conversations_stay_running_on_load() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            None,
        )
        .expect("conversation should be created");

        mark_turn_running(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
            "Keep running in the background",
        )
        .expect("turn should start");

        let _guard = track_running_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should be tracked");

        let loaded = get_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should load");

        assert_eq!(loaded.summary.status, ConversationStatus::Running);
        assert_eq!(loaded.summary.error, None);
    }

    #[test]
    fn deletes_non_running_conversation() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Delete me"),
        )
        .expect("conversation should be created");

        delete_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should delete");

        let listed = list_conversations(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            false,
        )
        .expect("conversations should list");
        assert!(listed.is_empty());
    }

    #[test]
    fn renames_conversation() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Original title"),
        )
        .expect("conversation should be created");

        let renamed = rename_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
            "Renamed conversation",
        )
        .expect("conversation should rename");

        assert_eq!(renamed.title, "Renamed conversation");
    }

    #[test]
    fn archives_conversation_and_hides_it_from_listing() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Archive me"),
        )
        .expect("conversation should be created");

        let archived = archive_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should archive");

        assert!(archived.archived_at.is_some());

        let listed = list_conversations(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            false,
        )
        .expect("conversations should list");
        assert!(listed.is_empty());
    }

    #[test]
    fn includes_archived_conversations_when_requested() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Archive but keep visible"),
        )
        .expect("conversation should be created");

        let archived = archive_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should archive");

        let listed = list_conversations(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            true,
        )
        .expect("conversations should list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, archived.id);
        assert!(listed[0].archived_at.is_some());
    }

    #[test]
    fn unarchives_conversation_and_returns_it_to_default_listing() {
        let workspace = TestWorkspace::new();
        let conversation = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Bring me back"),
        )
        .expect("conversation should be created");

        archive_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should archive");

        let unarchived = unarchive_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &conversation.summary.id,
        )
        .expect("conversation should unarchive");

        assert!(unarchived.archived_at.is_none());

        let listed = list_conversations(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            false,
        )
        .expect("conversations should list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, conversation.summary.id);
    }

    #[test]
    fn pins_conversation_and_lists_it_first() {
        let workspace = TestWorkspace::new();
        let first = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("First conversation"),
        )
        .expect("first conversation should be created");

        let second = create_conversation(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            AgentSelection {
                provider: "codex".to_string(),
                model: "gpt-5.4".to_string(),
            },
            Some("Second conversation"),
        )
        .expect("second conversation should be created");

        let pinned = set_conversation_pinned(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            &first.summary.id,
            true,
        )
        .expect("conversation should pin");

        assert!(pinned.pinned_at.is_some());

        let listed = list_conversations(
            workspace
                .path()
                .to_str()
                .expect("workspace path should be utf-8"),
            false,
        )
        .expect("conversations should list");

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, first.summary.id);
        assert_eq!(listed[1].id, second.summary.id);
    }
}
