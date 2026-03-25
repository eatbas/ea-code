use crate::models::SessionMeta;

use super::{atomic_write, now_rfc3339, validate_id, with_session_lock, workspace_data_dir};

/// Returns the session directory under the workspace.
pub fn session_dir(workspace_path: &str, session_id: &str) -> Result<std::path::PathBuf, String> {
    validate_id(session_id)?;
    Ok(workspace_data_dir(workspace_path)?
        .join("sessions")
        .join(session_id))
}

/// Returns the session.json file path.
fn session_path(workspace_path: &str, session_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(session_dir(workspace_path, session_id)?.join("session.json"))
}

/// Creates a new session with the given metadata.
/// H8: Protected by file lock for concurrent access.
pub fn create_session(workspace_path: &str, meta: &SessionMeta) -> Result<(), String> {
    validate_id(&meta.id)?;
    with_session_lock(|| {
        let dir = session_dir(workspace_path, &meta.id)?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create session directory: {e}"))?;

        let path = dir.join("session.json");
        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;

        atomic_write(&path, &json)
    })
}

/// Reads a session's metadata.
pub fn read_session(workspace_path: &str, session_id: &str) -> Result<SessionMeta, String> {
    validate_id(session_id)?;
    let path = session_path(workspace_path, session_id)?;

    if !path.exists() {
        return Err(format!("Session not found: {session_id}"));
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read session file: {e}"))?;

    let meta: SessionMeta = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse session file: {e}"))?;

    Ok(meta)
}

/// Updates a session's metadata (atomically).
/// H8: Protected by file lock for concurrent access.
pub fn update_session(workspace_path: &str, meta: &SessionMeta) -> Result<(), String> {
    validate_id(&meta.id)?;
    with_session_lock(|| {
        let path = session_path(workspace_path, &meta.id)?;

        if !path.exists() {
            return Err(format!("Session not found: {}", meta.id));
        }

        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;

        atomic_write(&path, &json)
    })
}

/// Lists sessions for a workspace by scanning its sessions directory.
/// Returns sessions sorted by updated_at descending (most recent first).
pub fn list_sessions(workspace_path: &str) -> Result<Vec<SessionMeta>, String> {
    let sessions_dir = workspace_data_dir(workspace_path)?.join("sessions");

    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = scan_sessions_dir(&sessions_dir)?;
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(sessions)
}

/// Lists ALL sessions across all known workspaces (for crash recovery).
/// Returns sessions sorted by updated_at descending (most recent first).
pub fn list_all_sessions() -> Result<Vec<SessionMeta>, String> {
    let projects = super::projects::read_projects()?;
    let mut all_sessions = Vec::new();

    for project in &projects {
        let ws = std::path::Path::new(&project.path);
        if !ws.exists() {
            continue;
        }
        let sessions_dir = ws.join(".ea-code").join("sessions");
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
pub fn delete_session(workspace_path: &str, session_id: &str) -> Result<(), String> {
    validate_id(session_id)?;
    with_session_lock(|| {
        let dir = session_dir(workspace_path, session_id)?;

        if !dir.exists() {
            return Err(format!("Session not found: {session_id}"));
        }

        std::fs::remove_dir_all(&dir)
            .map_err(|e| format!("Failed to delete session directory: {e}"))?;

        Ok(())
    })
}

/// Increment run count - called ONCE during create_run.
/// H8: Protected by file lock for concurrent access.
pub fn increment_run_count(workspace_path: &str, session_id: &str) -> Result<(), String> {
    validate_id(session_id)?;
    with_session_lock(|| {
        let mut meta = read_session(workspace_path, session_id)?;
        meta.run_count += 1;

        let path = session_path(workspace_path, &meta.id)?;
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| format!("Failed to serialise session: {e}"))?;
        atomic_write(&path, &json)
    })
}

/// Touch session - updates timestamps and metadata, does NOT increment run_count.
/// H8: Protected by file lock for concurrent access.
pub fn touch_session(
    workspace_path: &str,
    session_id: &str,
    last_prompt: Option<&str>,
    last_status: Option<&str>,
    last_verdict: Option<&str>,
) -> Result<(), String> {
    validate_id(session_id)?;
    with_session_lock(|| {
        let mut meta = read_session(workspace_path, session_id)?;

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

        let path = session_path(workspace_path, &meta.id)?;
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
