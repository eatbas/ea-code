use crate::models::AppSettings;

use super::{atomic_write, config_dir, with_settings_lock};

const SETTINGS_FILE: &str = "settings.json";
const SCHEMA_VERSION: u32 = 1;

/// Wrapper for serialization that includes schema version.
#[derive(serde::Serialize, serde::Deserialize)]
struct SettingsWrapper {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(flatten)]
    settings: AppSettings,
}

/// Reads settings from settings.json.
/// Returns default settings if the file doesn't exist.
pub fn read_settings() -> Result<AppSettings, String> {
    with_settings_lock(|| {
        let path = config_dir()?.join(SETTINGS_FILE);

        if !path.exists() {
            return Ok(AppSettings::default());
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read settings file: {e}"))?;

        // Try to parse with wrapper first (new format)
        if let Ok(wrapper) = serde_json::from_str::<SettingsWrapper>(&contents) {
            return Ok(wrapper.settings);
        }

        // Fall back to direct AppSettings parsing (for compatibility)
        let settings: AppSettings = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse settings file: {e}"))?;

        Ok(settings)
    })
}

/// Writes settings to settings.json atomically.
pub fn write_settings(settings: &AppSettings) -> Result<(), String> {
    with_settings_lock(|| {
        let path = config_dir()?.join(SETTINGS_FILE);

        let wrapper = SettingsWrapper {
            schema_version: SCHEMA_VERSION,
            settings: settings.clone(),
        };

        let json = serde_json::to_string_pretty(&wrapper)
            .map_err(|e| format!("Failed to serialise settings: {e}"))?;

        atomic_write(&path, &json)
    })
}

/// One-time migration from legacy JSON settings file.
pub fn import_from_legacy_json() -> Result<(), String> {
    with_settings_lock(|| {
        let settings_path = config_dir()?.join(SETTINGS_FILE);

        if settings_path.exists() {
            return Ok(());
        }

        let legacy_json_path = config_dir()?.join("settings.json.legacy");
        if legacy_json_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&legacy_json_path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&contents) {
                    let wrapper = SettingsWrapper {
                        schema_version: SCHEMA_VERSION,
                        settings,
                    };
                    let json = serde_json::to_string_pretty(&wrapper)
                        .map_err(|e| format!("Failed to serialise settings: {e}"))?;
                    atomic_write(&settings_path, &json)?;

                    let _ = std::fs::rename(
                        &legacy_json_path,
                        legacy_json_path.with_extension("imported"),
                    );
                    return Ok(());
                }
            }
        }

        Ok(())
    })
}
