use tauri::State;

use crate::db;
use crate::models::AppSettings;

use super::AppState;

/// Returns the current application settings from the database.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    db::settings::get(&state.db)
}

/// Persists application settings to the database.
#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    new_settings: AppSettings,
) -> Result<(), String> {
    db::settings::update(&state.db, &new_settings)
}

/// Returns merged settings (global + project overrides) for a workspace.
#[tauri::command]
pub async fn get_project_settings(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<AppSettings, String> {
    db::settings::get_merged_for_workspace(&state.db, &project_path)
}

/// Persists project-specific settings overrides for a workspace.
#[tauri::command]
pub async fn save_project_settings(
    state: State<'_, AppState>,
    project_path: String,
    new_settings: AppSettings,
) -> Result<(), String> {
    db::settings::save_project_overrides_for_workspace(&state.db, &project_path, &new_settings)
}

/// Clears project-specific overrides for a workspace.
#[tauri::command]
pub async fn clear_project_settings(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<(), String> {
    db::settings::clear_project_overrides_for_workspace(&state.db, &project_path)
}
