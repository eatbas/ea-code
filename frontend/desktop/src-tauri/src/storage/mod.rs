/// File-based storage module replacing SQLite database.
///
/// Uses atomic writes (write to .tmp, then rename) for all JSON files.
/// Uses append-only JSONL for event logs.
pub mod cleanup;
pub mod index;
pub mod mcp;
pub mod messages;
pub mod migration;
pub mod projects;
pub mod recovery;
pub mod runs;
pub mod sessions;
pub mod settings;
pub mod skills;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

// Re-export index functions for backward compatibility.
pub use index::{add_run_to_index, get_session_for_run, remove_run_from_index};

// H8: Per-file-type locks for read-modify-write operations
lazy_static::lazy_static! {
    static ref SETTINGS_LOCK: Mutex<()> = Mutex::new(());
    static ref PROJECTS_LOCK: Mutex<()> = Mutex::new(());
    static ref SESSION_LOCK: Mutex<()> = Mutex::new(());
    static ref MCP_LOCK: Mutex<()> = Mutex::new(());
    static ref SKILLS_LOCK: Mutex<()> = Mutex::new(());
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

/// Helper to acquire lock for session file operations
pub fn with_session_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = SESSION_LOCK.lock().map_err(|_| "Session lock poisoned")?;
    f()
}

/// Helper to acquire lock for MCP config file operations
pub fn with_mcp_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = MCP_LOCK.lock().map_err(|_| "MCP lock poisoned")?;
    f()
}

/// Helper to acquire lock for skills file operations
pub fn with_skills_lock<T, F: FnOnce() -> Result<T, String>>(f: F) -> Result<T, String> {
    let _guard = SKILLS_LOCK.lock().map_err(|_| "Skills lock poisoned")?;
    f()
}

/// Returns the config directory: `~/.ea-code/`
pub fn config_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Unable to determine home directory".to_string())?;
    Ok(home.join(".ea-code"))
}

/// Atomically writes content to a file.
/// N7: Uses 3-step pattern with .bak file to prevent data loss on crash.
/// 1. Write to .tmp
/// 2. Rename original to .bak (if exists)
/// 3. Rename .tmp to target
/// 4. Delete .bak on success
pub fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
    }

    // H12: Proper temp file naming - append .tmp/.bak instead of replacing extension
    let path_str = path
        .as_os_str()
        .to_str()
        .ok_or_else(|| "Invalid path encoding".to_string())?;
    let tmp_path = PathBuf::from(format!("{}.tmp", path_str));
    let bak_path = PathBuf::from(format!("{}.bak", path_str));

    // Step 1: Write to temp file
    std::fs::write(&tmp_path, contents)
        .map_err(|e| format!("Failed to write tmp file {}: {e}", tmp_path.display()))?;

    // Step 2: If target exists, move it to .bak
    if path.exists() {
        std::fs::rename(path, &bak_path)
            .map_err(|e| format!("Failed to create backup {}: {e}", bak_path.display()))?;
    }

    // Step 3: Rename temp to target
    match std::fs::rename(&tmp_path, path) {
        Ok(()) => {
            // Step 4: Delete backup on success
            let _ = std::fs::remove_file(&bak_path);
            Ok(())
        }
        Err(e) => {
            // Restore backup on failure
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
/// If a .bak file exists without its corresponding target, restore it.
/// This handles crashes that occurred during atomic_write.
pub fn recover_orphaned_backups() -> Result<(), String> {
    let base = config_dir()?;

    // Scan for .bak files recursively
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
                    // Found a .bak file - check if target is missing
                    let target_path = path.with_extension("");
                    if !target_path.exists() {
                        // Orphaned backup - restore it
                        std::fs::rename(&path, &target_path).map_err(|e| {
                            format!(
                                "Failed to restore backup {} to {}: {e}",
                                path.display(),
                                target_path.display()
                            )
                        })?;
                        restored += 1;
                    } else {
                        // Target exists, backup is stale - delete it
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

/// Ensures all required directories exist.
pub fn ensure_dirs() -> Result<(), String> {
    let base = config_dir()?;

    std::fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;
    std::fs::create_dir_all(base.join("skills"))
        .map_err(|e| format!("Failed to create skills directory: {e}"))?;
    std::fs::create_dir_all(base.join("projects"))
        .map_err(|e| format!("Failed to create projects directory: {e}"))?;
    std::fs::create_dir_all(base.join("prompts"))
        .map_err(|e| format!("Failed to create prompts directory: {e}"))?;

    Ok(())
}

/// Returns the current UTC timestamp as an RFC 3339 string.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Validates an ID to prevent path traversal attacks.
/// Rejects IDs containing path separators or parent directory references.
pub fn validate_id(id: &str) -> Result<(), String> {
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err("Invalid ID: path traversal detected".to_string());
    }
    Ok(())
}
