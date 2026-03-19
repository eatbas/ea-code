//! Combined storage index for O(1) lookups.
//!
//! Maps `session_id → project_id` and `run_id → session_id`.
//! Persisted to `~/.ea-code/index.json` and cached in memory.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::{atomic_write, config_dir};

/// On-disk + in-memory index structure.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct StorageIndex {
    /// session_id → project_id
    #[serde(default)]
    pub sessions: HashMap<String, String>,
    /// run_id → session_id
    #[serde(default)]
    pub runs: HashMap<String, String>,
}

/// In-memory cache of the index (lazy-loaded).
static INDEX_CACHE: Mutex<Option<StorageIndex>> = Mutex::new(None);

/// File-level lock for index read-modify-write operations.
static INDEX_LOCK: Mutex<()> = Mutex::new(());

/// Acquires the index file lock.
fn with_index_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = INDEX_LOCK.lock().map_err(|_| "Index lock poisoned")?;
    f()
}

/// Returns the path to the index file.
fn index_path() -> Result<std::path::PathBuf, String> {
    Ok(config_dir()?.join("index.json"))
}

// ---- Internal (caller must hold lock) ----

fn load_unlocked() -> Result<StorageIndex, String> {
    // Try cache first
    if let Ok(cache) = INDEX_CACHE.lock() {
        if let Some(ref idx) = *cache {
            return Ok(idx.clone());
        }
    }

    let path = index_path()?;
    if !path.exists() {
        // Check for legacy run_index.json and migrate
        let legacy = config_dir()?.join("run_index.json");
        if legacy.exists() {
            return migrate_legacy_run_index(&legacy);
        }
        return Ok(StorageIndex::default());
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read index: {e}"))?;

    let idx: StorageIndex = serde_json::from_str(&contents).unwrap_or_default();

    // Update cache
    if let Ok(mut cache) = INDEX_CACHE.lock() {
        *cache = Some(idx.clone());
    }

    Ok(idx)
}

fn save_unlocked(idx: &StorageIndex) -> Result<(), String> {
    let path = index_path()?;
    let contents =
        serde_json::to_string_pretty(idx).map_err(|e| format!("Failed to serialise index: {e}"))?;

    atomic_write(&path, &contents)?;

    // Update cache
    if let Ok(mut cache) = INDEX_CACHE.lock() {
        *cache = Some(idx.clone());
    }

    Ok(())
}

/// Migrates from the old flat `run_index.json` (run_id → session_id) to the new format.
fn migrate_legacy_run_index(legacy_path: &std::path::Path) -> Result<StorageIndex, String> {
    let contents = std::fs::read_to_string(legacy_path)
        .map_err(|e| format!("Failed to read legacy run index: {e}"))?;

    let runs: HashMap<String, String> = serde_json::from_str(&contents).unwrap_or_default();

    let idx = StorageIndex {
        sessions: HashMap::new(),
        runs,
    };

    // Save in new format
    let path = index_path()?;
    let json = serde_json::to_string_pretty(&idx)
        .map_err(|e| format!("Failed to serialise index: {e}"))?;
    atomic_write(&path, &json)?;

    // Remove legacy file
    let _ = std::fs::remove_file(legacy_path);

    if let Ok(mut cache) = INDEX_CACHE.lock() {
        *cache = Some(idx.clone());
    }

    Ok(idx)
}

// ---- Public API ----

/// Loads the full index (with lock).
pub fn load() -> Result<StorageIndex, String> {
    with_index_lock(load_unlocked)
}

/// Saves the full index (with lock).
pub fn save(idx: &StorageIndex) -> Result<(), String> {
    with_index_lock(|| save_unlocked(idx))
}

/// Adds a session → project mapping.
pub fn add_session_to_index(session_id: &str, project_id: &str) -> Result<(), String> {
    with_index_lock(|| {
        let mut idx = load_unlocked()?;
        idx.sessions
            .insert(session_id.to_string(), project_id.to_string());
        save_unlocked(&idx)
    })
}

/// Returns the project_id for a session.
pub fn get_project_for_session(session_id: &str) -> Result<String, String> {
    let idx = load()?;
    idx.sessions
        .get(session_id)
        .cloned()
        .ok_or_else(|| format!("Session not found in index: {session_id}"))
}

/// Removes a session and all its runs from the index.
pub fn remove_session_from_index(session_id: &str) -> Result<(), String> {
    with_index_lock(|| {
        let mut idx = load_unlocked()?;
        idx.sessions.remove(session_id);
        // Remove all runs belonging to this session
        idx.runs.retain(|_run_id, sid| sid != session_id);
        save_unlocked(&idx)
    })
}

/// Adds a run → session mapping.
pub fn add_run_to_index(run_id: &str, session_id: &str) -> Result<(), String> {
    with_index_lock(|| {
        let mut idx = load_unlocked()?;
        idx.runs.insert(run_id.to_string(), session_id.to_string());
        save_unlocked(&idx)
    })
}

/// Returns the session_id for a run.
pub fn get_session_for_run(run_id: &str) -> Result<String, String> {
    let idx = load()?;
    idx.runs
        .get(run_id)
        .cloned()
        .ok_or_else(|| format!("Run not found: {run_id}"))
}

/// Removes a run from the index.
pub fn remove_run_from_index(run_id: &str) -> Result<(), String> {
    with_index_lock(|| {
        let mut idx = load_unlocked()?;
        idx.runs.remove(run_id);
        save_unlocked(&idx)
    })
}

/// Invalidates the in-memory cache (useful after migration rebuilds index).
pub fn invalidate_cache() {
    if let Ok(mut cache) = INDEX_CACHE.lock() {
        *cache = None;
    }
}
