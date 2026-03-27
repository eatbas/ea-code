/// File-based storage module.
///
/// Global data (settings, MCP) lives under `~/.ea-code/`.
/// Uses atomic writes (write to .tmp, then rename) for all JSON files.
pub mod mcp;
pub mod projects;
pub mod settings;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

// Per-file-type locks for read-modify-write operations
lazy_static::lazy_static! {
    static ref SETTINGS_LOCK: Mutex<()> = Mutex::new(());
    static ref PROJECTS_LOCK: Mutex<()> = Mutex::new(());
    static ref MCP_LOCK: Mutex<()> = Mutex::new(());
}

/// Helper to acquire lock for settings file operations
pub fn with_settings_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = SETTINGS_LOCK.lock().map_err(|_| "Settings lock poisoned")?;
    f()
}

/// Helper to acquire lock for projects file operations
pub fn with_projects_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = PROJECTS_LOCK.lock().map_err(|_| "Projects lock poisoned")?;
    f()
}

/// Helper to acquire lock for MCP config file operations
pub fn with_mcp_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = MCP_LOCK.lock().map_err(|_| "MCP lock poisoned")?;
    f()
}

/// Returns the config directory: `~/.ea-code/`
pub fn config_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Unable to determine home directory".to_string())?;
    Ok(home.join(".ea-code"))
}

/// Atomically writes content to a file.
/// N7: Uses 3-step pattern with .bak file to prevent data loss on crash.
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }

    let path_str = path
        .as_os_str()
        .to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())?;
    let tmp_path = PathBuf::from(format!("{}.tmp", path_str));
    let bak_path = PathBuf::from(format!("{}.bak", path_str));

    std::fs::write(&tmp_path, contents)
        .map_err(|e| format!("Failed to write tmp file {}: {e}", tmp_path.display()))?;

    if path.exists() {
        std::fs::rename(path, &bak_path)
            .map_err(|e| format!("Failed to create backup {}: {e}", bak_path.display()))?;
    }

    match std::fs::rename(&tmp_path, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(&bak_path);
            Ok(())
        }
        Err(e) => {
            if bak_path.exists() {
                let _ = std::fs::rename(&bak_path, path);
            }
            let _ = std::fs::remove_file(&tmp_path);
            Err(format!(
                "Failed to rename {} to {}: {e}",
                tmp_path.display(),
                path.display()
            ))
        }
    }
}

/// Recover any orphaned .bak files on startup.
pub fn recover_orphaned_backups() -> Result<(), String> {
    let base = config_dir()?;

    fn scan_and_restore(dir: &Path) -> Result<usize, String> {
        let mut restored = 0;
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {e}", dir.display()))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
            let path = entry.path();

            if path.is_dir() {
                restored += scan_and_restore(&path)?;
            } else if let Some(ext) = path.extension() {
                if ext == "bak" {
                    let target_path = path.with_extension("");
                    if !target_path.exists() {
                        std::fs::rename(&path, &target_path).map_err(|e| {
                            format!(
                                "Failed to restore backup {} to {}: {e}",
                                path.display(),
                                target_path.display()
                            )
                        })?;
                        restored += 1;
                    } else {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }

        Ok(restored)
    }

    let restored = scan_and_restore(&base)?;
    if restored > 0 {
        eprintln!("Recovered {restored} orphaned backup file(s)");
    }

    Ok(())
}

/// Ensures global config directories exist under `~/.ea-code/`.
pub fn ensure_dirs() -> Result<(), String> {
    let base = config_dir()?;

    std::fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;

    Ok(())
}

/// Returns the current UTC timestamp as an RFC 3339 string.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
