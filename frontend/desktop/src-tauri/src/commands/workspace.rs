use crate::models::*;
use tauri::{AppHandle, State};

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

    let info = crate::git::workspace_info(&path).await;
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

/// Checks the health of all configured CLI tools.
#[tauri::command]
pub async fn validate_environment(settings: AppSettings) -> Result<CliHealth, String> {
    check_cli_health_inner(&settings).await
}

/// Opens the given workspace path in VS Code.
#[tauri::command]
pub async fn open_in_vscode(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    app.shell()
        .command("code")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open VS Code: {e}"))?;
    Ok(())
}
