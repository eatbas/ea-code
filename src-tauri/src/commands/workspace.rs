use crate::models::*;
use tauri::State;

use super::cli::check_cli_health_inner;
use super::AppState;

/// Validates a workspace directory and returns its git status.
#[tauri::command]
pub async fn select_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<WorkspaceInfo, String> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("Cannot access path: {e}"))?;

    if !meta.is_dir() {
        return Err("Selected path is not a directory".to_string());
    }

    let info = crate::git::workspace_info(&path);
    let workspace_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|os| os.to_str())
        .unwrap_or(&path);

    crate::db::projects::upsert(
        &state.db,
        &path,
        workspace_name,
        info.is_git_repo,
        info.branch.as_deref(),
    )?;

    Ok(info)
}

/// Refreshes git status for an already-selected workspace without database side effects.
#[tauri::command]
pub async fn refresh_workspace(path: String) -> Result<WorkspaceInfo, String> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("Cannot access path: {e}"))?;
    if !meta.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    Ok(crate::git::workspace_info(&path))
}

/// Checks the health of all configured CLI tools.
#[tauri::command]
pub async fn validate_environment(settings: AppSettings) -> Result<CliHealth, String> {
    check_cli_health_inner(&settings).await
}
