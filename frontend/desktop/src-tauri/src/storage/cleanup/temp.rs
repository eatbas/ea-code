use std::path::Path;

use crate::storage::config_dir;

/// Removes stale temporary files on app startup.
///
/// Safe to run only at startup when no pipeline is active.
pub fn cleanup_stale_temp_files() -> Result<usize, String> {
    let base = config_dir()?;
    let mut cleaned = 0usize;

    // 1. Delete dead .txt prompt files in {config_dir}/prompts/
    let prompts_dir = base.join("prompts");
    if prompts_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&prompts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("txt") {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            eprintln!("Cleanup: removed stale prompt file {}", path.display());
                            cleaned += 1;
                        }
                        Err(e) => {
                            eprintln!(
                                "Cleanup warning: could not remove {}: {e}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
    }

    // 2. Delete dead mcp-config-*.json in config dir root
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("mcp-config-") && name.ends_with(".json") {
                        match std::fs::remove_file(&path) {
                            Ok(()) => {
                                eprintln!(
                                    "Cleanup: removed stale MCP config {}",
                                    path.display()
                                );
                                cleaned += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Cleanup warning: could not remove {}: {e}",
                                    path.display()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Recursively delete orphaned .tmp files anywhere under config dir
    cleaned += remove_tmp_files_recursive(&base);

    // 4. Delete legacy SQLite files matching ea-code.db*
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("ea-code.db") {
                        match std::fs::remove_file(&path) {
                            Ok(()) => {
                                eprintln!(
                                    "Cleanup: removed legacy SQLite file {}",
                                    path.display()
                                );
                                cleaned += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Cleanup warning: could not remove {}: {e}",
                                    path.display()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // 5. Delete empty run directories (no summary.json) across all workspaces
    let projects = crate::storage::projects::read_projects().unwrap_or_default();
    for project in &projects {
        let ws = std::path::Path::new(&project.path);
        if !ws.exists() {
            continue;
        }
        let sessions_dir = ws.join(".ea-code").join("sessions");
        if !sessions_dir.is_dir() {
            continue;
        }
        if let Ok(session_entries) = std::fs::read_dir(&sessions_dir) {
            for session_entry in session_entries.flatten() {
                let runs_dir = session_entry.path().join("runs");
                if !runs_dir.is_dir() {
                    continue;
                }
                if let Ok(run_entries) = std::fs::read_dir(&runs_dir) {
                    for run_entry in run_entries.flatten() {
                        let run_path = run_entry.path();
                        if !run_path.is_dir() {
                            continue;
                        }
                        let summary = run_path.join("summary.json");
                        if !summary.exists() {
                            match std::fs::remove_dir_all(&run_path) {
                                Ok(()) => {
                                    eprintln!(
                                        "Cleanup: removed empty run dir {}",
                                        run_path.display()
                                    );
                                    cleaned += 1;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Cleanup warning: could not remove {}: {e}",
                                        run_path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if cleaned > 0 {
        eprintln!("Startup cleanup: removed {cleaned} stale file(s)/dir(s)");
    }

    Ok(cleaned)
}

/// Recursively removes `.tmp` files under `dir`, returning the count removed.
fn remove_tmp_files_recursive(dir: &Path) -> usize {
    let mut count = 0;
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            count += remove_tmp_files_recursive(&path);
        } else if path.is_file()
            && path.extension().and_then(|e| e.to_str()) == Some("tmp")
        {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    eprintln!("Cleanup: removed orphaned tmp file {}", path.display());
                    count += 1;
                }
                Err(e) => {
                    eprintln!(
                        "Cleanup warning: could not remove {}: {e}",
                        path.display()
                    );
                }
            }
        }
    }
    count
}
