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
