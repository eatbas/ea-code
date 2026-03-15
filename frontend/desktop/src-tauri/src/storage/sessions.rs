use crate::models::SessionMeta;

use super::{atomic_write, config_dir, now_rfc3339, validate_id, with_session_lock};
use super::index;

/// Returns the session directory path within a known project.
/// Used during creation when the session is not yet in the index.
pub fn session_dir_for_project(
    project_id: &str,
    session_id: &str,
) -> Result<std::path::PathBuf, String> {
    validate_id(project_id)?;
    validate_id(session_id)?;
    Ok(config_dir()?
        .join("projects")
        .join(project_id)
        .join("sessions")
        .join(session_id))
}

/// Returns the session directory path by looking up the project from the index.
/// Used for reads/updates after the session has been created.
pub fn session_dir(id: &str) -> Result<std::path::PathBuf, String> {
    validate_id(id)?;
    let project_id = index::get_project_for_session(id)?;
    session_dir_for_project(&project_id, id)
}

/// Returns the session.json file path.
fn session_path_for_project(project_id: &str, session_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(session_dir_for_project(project_id, session_id)?.join("session.json"))
}

/// Returns the session.json file path (via index lookup).
fn session_path(id: &str) -> Result<std::path::PathBuf, String> {
    Ok(session_dir(id)?.join("session.json"))
}

/// Creates a new session with the given metadata.
/// Writes to `projects/{project_id}/sessions/{session_id}/session.json`.
/// H8: Protected by file lock for concurrent access.
pub fn create_session(meta: &SessionMeta) -> Result<(), String> {
    validate_id(&meta.id)?;
    validate_id(&meta.project_id)?;
    with_session_lock(|| {
        let path = session_path_for_project(&meta.project_id, &meta.id)?;

        // Create session directory
        std::fs::create_dir_all(session_dir_for_project(&meta.project_id, &meta.id)?)
            .map_err(|e| format!("Failed to create session directory: {e}"))?;

        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;

        atomic_write(&path, &json)?;

        // Register in index
        index::add_session_to_index(&meta.id, &meta.project_id)?;

        Ok(())
    })
}

/// Reads a session's metadata.
pub fn read_session(id: &str) -> Result<SessionMeta, String> {
    validate_id(id)?;
    let path = session_path(id)?;

    if !path.exists() {
        return Err(format!("Session not found: {id}"));
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read session file: {e}"))?;

    let meta: SessionMeta = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse session file: {e}"))?;

    Ok(meta)
}

/// Updates a session's metadata (atomically).
/// H8: Protected by file lock for concurrent access.
pub fn update_session(meta: &SessionMeta) -> Result<(), String> {
    validate_id(&meta.id)?;
    with_session_lock(|| {
        let path = session_path(&meta.id)?;

        if !path.exists() {
            return Err(format!("Session not found: {}", meta.id));
        }

        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;

        atomic_write(&path, &json)
    })
}

/// Lists sessions for a specific project by scanning its sessions directory.
/// Returns sessions sorted by updated_at descending (most recent first).
pub fn list_sessions(project_id: &str) -> Result<Vec<SessionMeta>, String> {
    validate_id(project_id)?;
    let sessions_dir = config_dir()?
        .join("projects")
        .join(project_id)
        .join("sessions");

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = scan_sessions_dir(&sessions_dir)?;
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(sessions)
}

/// Lists ALL sessions across all projects (for crash recovery).
/// Returns sessions sorted by updated_at descending (most recent first).
pub fn list_all_sessions() -> Result<Vec<SessionMeta>, String> {
    let projects_dir = config_dir()?.join("projects");

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut all_sessions = Vec::new();

    let project_entries = std::fs::read_dir(&projects_dir)
        .map_err(|e| format!("Failed to read projects directory: {e}"))?;

    for project_entry in project_entries {
        let project_entry =
            project_entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let project_path = project_entry.path();

        if !project_path.is_dir() {
            continue;
        }

        let sessions_dir = project_path.join("sessions");
        if sessions_dir.exists() {
            if let Ok(sessions) = scan_sessions_dir(&sessions_dir) {
                all_sessions.extend(sessions);
            }
        }
    }

    all_sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(all_sessions)
}

/// Scans a sessions directory and returns all valid session metadata.
fn scan_sessions_dir(sessions_dir: &std::path::Path) -> Result<Vec<SessionMeta>, String> {
    let mut sessions = Vec::new();

    let entries = std::fs::read_dir(sessions_dir)
        .map_err(|e| format!("Failed to read sessions directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_dir() {
            let session_json = path.join("session.json");
            if session_json.exists() {
                match std::fs::read_to_string(&session_json) {
                    Ok(contents) => match serde_json::from_str::<SessionMeta>(&contents) {
                        Ok(meta) => sessions.push(meta),
                        Err(e) => eprintln!(
                            "Warning: Failed to parse session file {}: {e}",
                            session_json.display()
                        ),
                    },
                    Err(e) => eprintln!(
                        "Warning: Failed to read session file {}: {e}",
                        session_json.display()
                    ),
                }
            }
        }
    }

    Ok(sessions)
}

/// Deletes a session and all its runs (recursive delete).
/// H8: Protected by file lock for concurrent access.
pub fn delete_session(id: &str) -> Result<(), String> {
    validate_id(id)?;
    with_session_lock(|| {
        let dir = session_dir(id)?;

        if !dir.exists() {
            return Err(format!("Session not found: {id}"));
        }

        std::fs::remove_dir_all(&dir)
            .map_err(|e| format!("Failed to delete session directory: {e}"))?;

        // Remove from index (also removes all associated runs)
        index::remove_session_from_index(id)?;

        Ok(())
    })
}

/// Increment run count - called ONCE during create_run.
/// H8: Protected by file lock for concurrent access.
pub fn increment_run_count(session_id: &str) -> Result<(), String> {
    validate_id(session_id)?;
    with_session_lock(|| {
        let mut meta = read_session(session_id)?;
        meta.run_count += 1;

        // Update directly inside lock to avoid deadlock
        let path = session_path(&meta.id)?;
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;
        atomic_write(&path, &json)
    })
}

/// Touch session - updates timestamps and metadata, does NOT increment run_count.
/// H8: Protected by file lock for concurrent access.
pub fn touch_session(
    session_id: &str,
    last_prompt: Option<&str>,
    last_status: Option<&str>,
    last_verdict: Option<&str>,
) -> Result<(), String> {
    validate_id(session_id)?;
    with_session_lock(|| {
        let mut meta = read_session(session_id)?;

        meta.updated_at = now_rfc3339();

        if let Some(prompt) = last_prompt {
            meta.last_prompt = Some(prompt.to_string());
        }
        if let Some(status) = last_status {
            meta.last_status = Some(status.to_string());
        }
        if let Some(verdict) = last_verdict {
            meta.last_verdict = Some(verdict.to_string());
        }

        // Update directly inside lock to avoid deadlock
        let path = session_path(&meta.id)?;
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;
        atomic_write(&path, &json)
    })
}

/// Creates a new session metadata object with the current timestamp.
pub fn create_session_meta(
    id: String,
    title: String,
    project_path: String,
    project_id: String,
) -> SessionMeta {
    let now = now_rfc3339();
    SessionMeta {
        id,
        title,
        project_id,
        project_path,
        run_count: 0,
        last_prompt: None,
        last_status: None,
        last_verdict: None,
        created_at: now.clone(),
        updated_at: now,
    }
}
