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

/// Returns full session detail with all runs for the ChatView.
#[tauri::command]
pub async fn get_session_detail(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<db::models::SessionDetail, String> {
    let session = db::sessions::get_by_id(&state.db, &session_id)?
        .ok_or_else(|| "Session not found".to_string())?;

    let project_path = db::sessions::get_project_path(&state.db, &session_id)?;

    let run_summaries = db::runs::list_for_session(&state.db, &session_id)?;
    let mut run_details = Vec::with_capacity(run_summaries.len());
    for rs in &run_summaries {
        run_details.push(db::runs::get_full(&state.db, &rs.id)?);
    }

    Ok(db::models::SessionDetail {
        id: session.id,
        title: session.title,
        project_path,
        created_at: session.created_at,
        updated_at: session.updated_at,
        runs: run_details,
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
    db::runs::get_full(&state.db, &run_id)
}

/// Returns paginated logs for a run.
#[tauri::command]
pub async fn get_run_logs(
    state: State<'_, AppState>,
    run_id: String,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<db::models::LogRow>, String> {
    db::logs::get_for_run(
        &state.db,
        &run_id,
        offset.unwrap_or(0),
        limit.unwrap_or(500),
    )
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
