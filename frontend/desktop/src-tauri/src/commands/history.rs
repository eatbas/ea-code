use tauri::State;

use crate::db;

use super::AppState;

/// Returns recently opened projects for the sidebar.
#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
) -> Result<Vec<db::models::ProjectRow>, String> {
    db::projects::list_recent(&state.db, 20)
}

/// Returns session threads for a given project path.
#[tauri::command]
pub async fn list_sessions(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<Vec<db::models::SessionSummary>, String> {
    let project = db::projects::get_by_path(&state.db, &project_path)?
        .ok_or_else(|| "Project not found".to_string())?;
    db::sessions::list_for_project(&state.db, project.id, 50)
}

/// Returns paginated session detail with batch-loaded runs for the ChatView.
///
/// Loads the most recent `limit` runs (default 20), offset from the newest.
/// Returns them in chronological order (oldest first) along with `total_runs`
/// so the frontend can show a "Load earlier runs" button.
#[tauri::command]
pub async fn get_session_detail(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<db::models::SessionDetail, String> {
    let session = db::sessions::get_by_id(&state.db, &session_id)?
        .ok_or_else(|| "Session not found".to_string())?;

    let project_path = db::sessions::get_project_path(&state.db, &session_id)?;

    let total_runs = db::run_detail::count_for_session(&state.db, &session_id)?;
    let run_details = db::run_detail::list_full_for_session(
        &state.db,
        &session_id,
        limit.unwrap_or(20),
        offset.unwrap_or(0),
    )?;

    Ok(db::models::SessionDetail {
        id: session.id,
        title: session.title,
        project_path,
        created_at: session.created_at,
        updated_at: session.updated_at,
        runs: run_details,
        total_runs,
    })
}

/// Creates a new session thread for a project.
#[tauri::command]
pub async fn create_session(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<String, String> {
    let project = db::projects::get_by_path(&state.db, &project_path)?
        .ok_or_else(|| "Project not found".to_string())?;

    let session_id = uuid::Uuid::new_v4().to_string();
    db::sessions::create(&state.db, &session_id, project.id, "New Session")?;

    Ok(session_id)
}

/// Returns full detail for a single run.
#[tauri::command]
pub async fn get_run_detail(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<db::models::RunDetail, String> {
    db::run_detail::get_full(&state.db, &run_id)
}

/// Returns artefacts for a run.
#[tauri::command]
pub async fn get_run_artifacts(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<db::models::ArtifactRow>, String> {
    db::artifacts::get_for_run(&state.db, &run_id)
}

/// Deletes a session and all associated data.
#[tauri::command]
pub async fn delete_session(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    db::sessions::delete(&state.db, &session_id)
}
