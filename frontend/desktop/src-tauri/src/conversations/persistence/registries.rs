use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::storage::{atomic_write, with_conversations_lock};

fn running_conversations() -> &'static Mutex<HashSet<String>> {
    static ACTIVE_RUNNING_CONVERSATIONS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ACTIVE_RUNNING_CONVERSATIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(super) fn running_conversation_key(workspace_path: &str, conversation_id: &str) -> String {
    format!("{workspace_path}::{conversation_id}")
}

pub(super) fn is_running_conversation_tracked(
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
    workspace_path: String,
    conversation_id: String,
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
        if let Err(error) = persist_running_remove(&self.workspace_path, &self.conversation_id) {
            eprintln!(
                "[registries] Failed to clear persisted running flag for {}: {error}",
                self.conversation_id
            );
        }
    }
}

pub fn track_running_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<RunningConversationGuard, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let inserted = running_conversations()
        .lock()
        .map_err(|error| format!("Failed to track running conversation: {error}"))?
        .insert(key.clone());
    if !inserted {
        return Err("Conversation is already running".to_string());
    }
    // Persist alongside the in-memory set so a crash or restart leaves a durable
    // record that the reattach pass can consult to reconnect to Symphony.
    if let Err(error) = persist_running_add(workspace_path, conversation_id) {
        // Roll back the in-memory insert if the file write failed so the two
        // views stay consistent.
        if let Ok(mut tracked) = running_conversations().lock() {
            tracked.remove(&key);
        }
        return Err(format!(
            "Failed to persist running conversation flag: {error}"
        ));
    }
    Ok(RunningConversationGuard {
        key,
        workspace_path: workspace_path.to_string(),
        conversation_id: conversation_id.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Persistent running-conversation registry — survives crashes and restarts so
// the startup reattach pass can reconnect to Symphony scores that were still
// running when the app went down.
// ---------------------------------------------------------------------------

fn running_file_path(workspace_path: &str) -> PathBuf {
    Path::new(workspace_path)
        .join(".maestro")
        .join("running.json")
}

fn read_running_file_unlocked(workspace_path: &str) -> HashSet<String> {
    let path = running_file_path(workspace_path);
    let Ok(data) = std::fs::read_to_string(&path) else {
        return HashSet::new();
    };
    serde_json::from_str::<HashSet<String>>(&data).unwrap_or_default()
}

fn write_running_file_unlocked(workspace_path: &str, ids: &HashSet<String>) -> Result<(), String> {
    let path = running_file_path(workspace_path);
    if ids.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|error| format!("Failed to remove running file: {error}"))?;
        }
        return Ok(());
    }
    let mut sorted: Vec<&String> = ids.iter().collect();
    sorted.sort();
    let json = serde_json::to_string_pretty(&sorted)
        .map_err(|error| format!("Failed to serialise running set: {error}"))?;
    atomic_write(&path, &json)
}

fn persist_running_add(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    with_conversations_lock(|| {
        let mut set = read_running_file_unlocked(workspace_path);
        set.insert(conversation_id.to_string());
        write_running_file_unlocked(workspace_path, &set)
    })
}

fn persist_running_remove(workspace_path: &str, conversation_id: &str) -> Result<(), String> {
    with_conversations_lock(|| {
        let mut set = read_running_file_unlocked(workspace_path);
        if set.remove(conversation_id) {
            write_running_file_unlocked(workspace_path, &set)
        } else {
            Ok(())
        }
    })
}

/// Read the persisted set of conversations that were marked running at the
/// time of the last `track_running_conversation` / guard-drop. The reattach
/// pass consults this at startup to decide which Symphony scores to poll.
pub fn read_persisted_running_conversations(workspace_path: &str) -> Result<Vec<String>, String> {
    with_conversations_lock(|| {
        let mut ids: Vec<String> = read_running_file_unlocked(workspace_path)
            .into_iter()
            .collect();
        ids.sort();
        Ok(ids)
    })
}

/// Remove a conversation from the persisted running set without touching the
/// in-memory set. Called by the reattach pass once it has applied a terminal
/// state (or otherwise finished reconciling the conversation).
pub fn forget_persisted_running_conversation(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    persist_running_remove(workspace_path, conversation_id)
}

/// Return true if the conversation is in the persistent running set — i.e. it
/// was tracked as running when the last write happened. Used by the in-process
/// reconcile path to defer stale-marking until the reattach pass has had a
/// chance to query Symphony.
pub(super) fn is_running_conversation_persisted(
    workspace_path: &str,
    conversation_id: &str,
) -> bool {
    read_running_file_unlocked(workspace_path).contains(conversation_id)
}

// ---------------------------------------------------------------------------
// Abort flags
// ---------------------------------------------------------------------------

fn abort_flags() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static FLAGS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    FLAGS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_abort_flag(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Arc<AtomicBool>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let mut guard = abort_flags()
        .lock()
        .map_err(|error| format!("Failed to register abort flag: {error}"))?;
    if let Some(existing) = guard.get(&key) {
        return Ok(existing.clone());
    }

    let flag = Arc::new(AtomicBool::new(false));
    guard.insert(key, flag.clone());
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
// Pipeline score-ID registry — tracks live Symphony score IDs so stop_pipeline
// can cancel them on the server side.
// ---------------------------------------------------------------------------

fn pipeline_jobs() -> &'static Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<Option<String>>>>>> {
    static JOBS: OnceLock<Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<Option<String>>>>>>> =
        OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Pre-allocate empty score-ID slots for each planner. Call *before* spawning
/// the pipeline task so that `get_pipeline_score_ids` can read them even if
/// `stop_pipeline` is called immediately.
pub fn register_pipeline_score_slots(
    workspace_path: &str,
    conversation_id: &str,
    count: usize,
) -> Result<Vec<Arc<std::sync::Mutex<Option<String>>>>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let mut guard = pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to register pipeline score slots: {e}"))?;
    if let Some(existing) = guard.get(&key) {
        return Ok(existing.clone());
    }

    let slots: Vec<_> = (0..count)
        .map(|_| Arc::new(std::sync::Mutex::new(None)))
        .collect();
    guard.insert(key, slots.clone());
    Ok(slots)
}

/// Ensure a score-ID slot exists for an arbitrary stage index. Dynamic review
/// cycles append stages beyond the fixed pipeline layout, so they must join the
/// same shared registry as the original stages.
pub fn ensure_pipeline_score_slot(
    workspace_path: &str,
    conversation_id: &str,
    stage_index: usize,
) -> Result<Arc<std::sync::Mutex<Option<String>>>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let mut guard = pipeline_jobs()
        .lock()
        .map_err(|error| format!("Failed to ensure pipeline score slot: {error}"))?;
    let slots = guard.entry(key).or_default();
    while slots.len() <= stage_index {
        slots.push(Arc::new(std::sync::Mutex::new(None)));
    }
    Ok(slots[stage_index].clone())
}

/// Return every non-empty score ID currently registered for this pipeline.
pub fn get_pipeline_score_ids(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<String>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let guard = pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to read pipeline score slots: {e}"))?;
    let Some(slots) = guard.get(&key) else {
        return Ok(Vec::new());
    };
    Ok(slots
        .iter()
        .filter_map(|slot| slot.lock().ok().and_then(|g| g.clone()))
        .collect())
}

/// Remove all score-ID slots for a finished pipeline.
pub fn remove_pipeline_score_slots(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to remove pipeline score slots: {e}"))?
        .remove(&key);
    Ok(())
}

// ---------------------------------------------------------------------------
// Pipeline stage output buffers — accumulate live score output text on the Rust
// side so it survives frontend navigation (React state is ephemeral).
// ---------------------------------------------------------------------------

fn stage_buffers() -> &'static Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<String>>>>> {
    static BUFS: OnceLock<Mutex<HashMap<String, Vec<Arc<std::sync::Mutex<String>>>>>> =
        OnceLock::new();
    BUFS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Pre-allocate one text buffer per planner. Each planner appends its live
/// output here so `get_pipeline_state` can return accumulated text even
/// after the frontend navigated away and back.
pub fn register_pipeline_stage_buffers(
    workspace_path: &str,
    conversation_id: &str,
    count: usize,
) -> Result<Vec<Arc<std::sync::Mutex<String>>>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let mut guard = stage_buffers()
        .lock()
        .map_err(|e| format!("Failed to register stage buffers: {e}"))?;
    if let Some(existing) = guard.get(&key) {
        return Ok(existing.clone());
    }

    let buffers: Vec<_> = (0..count)
        .map(|_| Arc::new(std::sync::Mutex::new(String::new())))
        .collect();
    guard.insert(key, buffers.clone());
    Ok(buffers)
}

/// Ensure a stage buffer exists for an arbitrary stage index so dynamic stages
/// participate in the same reload/hydration path as the fixed layout.
pub fn ensure_pipeline_stage_buffer(
    workspace_path: &str,
    conversation_id: &str,
    stage_index: usize,
) -> Result<Arc<std::sync::Mutex<String>>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let mut guard = stage_buffers()
        .lock()
        .map_err(|error| format!("Failed to ensure stage buffer: {error}"))?;
    let buffers = guard.entry(key).or_default();
    while buffers.len() <= stage_index {
        buffers.push(Arc::new(std::sync::Mutex::new(String::new())));
    }
    Ok(buffers[stage_index].clone())
}

/// Read accumulated output text from every registered stage buffer keyed by its
/// actual stage index so sparse dynamic stages hydrate correctly.
pub fn get_pipeline_stage_texts(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<(usize, String)>, String> {
    let key = running_conversation_key(workspace_path, conversation_id);
    let guard = stage_buffers()
        .lock()
        .map_err(|e| format!("Failed to read stage buffers: {e}"))?;
    let Some(buffers) = guard.get(&key) else {
        return Ok(Vec::new());
    };
    Ok(buffers
        .iter()
        .enumerate()
        .map(|(index, buf)| (index, buf.lock().map(|g| g.clone()).unwrap_or_default()))
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
