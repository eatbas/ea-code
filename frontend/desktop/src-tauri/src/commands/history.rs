use crate::storage;
use crate::models::{SessionDetail, RunDetail, RunEvent};

/// Returns recently opened projects for the sidebar.
#[tauri::command]
pub async fn list_projects() -> Result<Vec<crate::models::ProjectEntry>, String> {
    storage::projects::read_projects()
}

/// Returns session threads for a given project path.
/// Resolves the filesystem path to a project_id, then lists sessions for that project.
#[tauri::command]
pub async fn list_sessions(project_path: String) -> Result<Vec<crate::models::SessionMeta>, String> {
    let project = match storage::projects::find_by_path(&project_path) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };
    storage::sessions::list_sessions(&project.id)
}

/// Returns paginated session detail with batch-loaded runs for the ChatView.
///
/// Loads the most recent `limit` runs (default 20), offset from the newest.
/// Returns them in chronological order (oldest first) along with `total_runs`
/// so the frontend can show a "Load earlier runs" button.
#[tauri::command]
pub async fn get_session_detail(
    session_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<SessionDetail, String> {
    let session = storage::sessions::read_session(&session_id)?;

    // Get all runs for this session
    let mut all_runs = storage::runs::list_runs(&session_id)?;

    // Sort by started_at descending (newest first) for pagination
    // This ensures we load the most recent runs when offset=0
    all_runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    let total_runs = all_runs.len() as u32;

    // Apply pagination
    let limit = limit.unwrap_or(20) as usize;
    let offset = offset.unwrap_or(0) as usize;

    let mut paginated_runs: Vec<_> = all_runs
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    // Reverse back to chronological order (oldest first) for display
    paginated_runs.reverse();

    Ok(SessionDetail {
        id: session.id,
        title: session.title,
        project_path: session.project_path,
        created_at: session.created_at,
        updated_at: session.updated_at,
        runs: paginated_runs,
        total_runs,
    })
}

/// Creates a new session thread for a project.
/// Resolves the filesystem path to a project_id to place the session correctly.
#[tauri::command]
pub async fn create_session(project_path: String) -> Result<String, String> {
    let project = storage::projects::find_by_path(&project_path)
        .ok_or_else(|| format!("Project not found for path: {project_path}"))?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let meta = storage::sessions::create_session_meta(
        session_id.clone(),
        "New Session".to_string(),
        project_path,
        project.id,
    );
    storage::sessions::create_session(&meta)?;
    Ok(session_id)
}

/// Returns full detail for a single run.
#[tauri::command]
pub async fn get_run_detail(run_id: String) -> Result<RunDetail, String> {
    let summary = storage::runs::read_summary(&run_id)?;
    let events = storage::runs::read_events(&run_id)?;
    Ok(RunDetail { summary, events })
}

/// Get events for a specific run (lazy loading).
#[tauri::command]
pub async fn get_run_events(run_id: String) -> Result<Vec<RunEvent>, String> {
    storage::runs::read_events(&run_id)
}

/// Returns all persisted artefacts for a run.
#[tauri::command]
pub async fn get_run_artifacts(
    run_id: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    storage::runs::read_all_artifacts(&run_id)
}

/// Deletes a session and all associated data.
#[tauri::command]
pub async fn delete_session(session_id: String) -> Result<(), String> {
    storage::sessions::delete_session(&session_id)
}
