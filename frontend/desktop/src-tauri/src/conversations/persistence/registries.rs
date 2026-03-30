use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

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
    let slots: Vec<_> = (0..count)
        .map(|_| Arc::new(std::sync::Mutex::new(None)))
        .collect();
    pipeline_jobs()
        .lock()
        .map_err(|e| format!("Failed to register pipeline score slots: {e}"))?
        .insert(key, slots.clone());
    Ok(slots)
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
