use crate::models::AppSettings;

use super::{atomic_write, config_dir, with_settings_lock};

const SETTINGS_FILE: &str = "settings.json";
const SCHEMA_VERSION: u32 = 1;

/// Wrapper for serialization that includes schema version.
///
/// This wrapper enables forward/backward compatibility by embedding a schema version
/// alongside the actual settings data. When reading, we first try to parse with the
/// wrapper (new format). If that fails, we fall back to parsing AppSettings directly
/// (legacy format without version). When writing, we always include the version.
///
/// This allows seamless migration: old files without version are read and then
/// rewritten with the version field on next save.
#[derive(serde::Serialize, serde::Deserialize)]
struct SettingsWrapper {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(flatten)]
    settings: AppSettings,
}

/// Reads settings from settings.json.
/// Returns default settings if the file doesn't exist.
/// N14: Protected by settings lock to prevent reading half-written file.
pub fn read_settings() -> Result<AppSettings, String> {
    with_settings_lock(|| {
        let path = config_dir()?.join(SETTINGS_FILE);

        if !path.exists() {
            return Ok(AppSettings::default());
        }

        let contents =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read settings file: {e}"))?;

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
/// H8: Protected by file lock for concurrent access.
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
///
/// This function ONLY handles JSON -> JSON migration (settings.json.legacy -> settings.json).
/// It does NOT read from the legacy SQLite database (ea-code.db).
/// SQLite migration would require the sqlite crate and proper DB schema handling.
///
/// Called on first launch if settings.json doesn't exist but a legacy file does.
/// H8: Protected by file lock for concurrent access.
pub fn import_from_legacy_json() -> Result<(), String> {
    with_settings_lock(|| {
        let settings_path = config_dir()?.join(SETTINGS_FILE);

        // Skip if settings.json already exists
        if settings_path.exists() {
            return Ok(());
        }

        // Try to read settings from legacy JSON file if it exists
        let legacy_json_path = config_dir()?.join("settings.json.legacy");
        if legacy_json_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&legacy_json_path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&contents) {
                    // Write settings directly inside lock to avoid deadlock
                    let wrapper = SettingsWrapper {
                        schema_version: SCHEMA_VERSION,
                        settings,
                    };
                    let json = serde_json::to_string_pretty(&wrapper)
                        .map_err(|e| format!("Failed to serialise settings: {e}"))?;
                    atomic_write(&settings_path, &json)?;

                    // Rename to prevent re-import
                    let _ = std::fs::rename(
                        &legacy_json_path,
                        legacy_json_path.with_extension("imported"),
                    );
                    return Ok(());
                }
            }
        }

        // No legacy settings to migrate - will use defaults
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.retention_days, 90);
        assert_eq!(settings.theme, "system");
    }
}
