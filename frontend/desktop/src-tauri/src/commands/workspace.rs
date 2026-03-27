use crate::models::*;
use crate::storage;
use tauri::AppHandle;

use super::cli::check_cli_health_inner;

/// Validates a workspace directory and returns its git status.
#[tauri::command]
pub async fn select_workspace(path: String) -> Result<WorkspaceInfo, String> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("Cannot access path: {e}"))?;

    if !meta.is_dir() {
        return Err("Selected path is not a directory".to_string());
    }

    let info = crate::git::workspace_info(&path).await;
    let workspace_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|os| os.to_str())
        .unwrap_or(&path);

    storage::projects::upsert(
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

/// Checks system-level prerequisites (Git Bash on Windows).
#[tauri::command]
pub async fn check_prerequisites() -> Result<PrerequisiteStatus, String> {
    // Git Bash check — only meaningful on Windows.
    let git_bash_available = if cfg!(target_os = "windows") {
        #[cfg(target_os = "windows")]
        {
            super::git_bash::find_git_bash().is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            true
        }
    } else {
        true
    };

    Ok(PrerequisiteStatus {
        python_available: true,
        python_version: None,
        git_bash_available,
        hive_api_source_found: false,
    })
}

/// Lists all known project bookmarks.
#[tauri::command]
pub async fn list_projects() -> Result<Vec<crate::models::ProjectEntry>, String> {
    storage::projects::list_projects()
}

/// Deletes a project bookmark by path.
#[tauri::command]
pub async fn delete_project(project_path: String) -> Result<(), String> {
    storage::projects::delete_project(&project_path)
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
