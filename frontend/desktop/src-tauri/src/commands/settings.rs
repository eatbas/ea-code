use crate::models::AppSettings;
use crate::storage;

/// Returns the current application settings from file storage.
#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    storage::settings::read_settings()
}

/// Persists application settings to file storage.
#[tauri::command]
pub async fn save_settings(new_settings: AppSettings) -> Result<(), String> {
    storage::settings::write_settings(&new_settings)
}
