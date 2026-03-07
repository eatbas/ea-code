use std::fs;
use std::path::PathBuf;

use crate::models::AppSettings;

/// Returns the path to the settings file:
/// `<config_dir>/ea-code/settings.json`
pub fn get_settings_path() -> Result<PathBuf, String> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| "Unable to determine config directory".to_string())?;
    Ok(config_dir.join("ea-code").join("settings.json"))
}

/// Loads settings from disk, falling back to defaults if the file is
/// missing or malformed.
pub fn load_settings() -> AppSettings {
    let path = match get_settings_path() {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };

    if !path.exists() {
        return AppSettings::default();
    }

    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

/// Serialises settings to JSON and writes them to disk, creating
/// intermediate directories if required.
pub fn save_settings_to_disk(settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialise settings: {e}"))?;

    fs::write(&path, json).map_err(|e| format!("Failed to write settings file: {e}"))?;

    Ok(())
}
