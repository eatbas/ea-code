//! One-time data migration from old flat storage layout to project hierarchy.
//!
//! Old: `sessions/{session_id}/...`
//! New: `projects/{project_id}/sessions/{session_id}/...`

use super::config_dir;

/// Migrates from old flat `sessions/` layout to `projects/{id}/sessions/{id}/` hierarchy.
///
/// Only acts if old `sessions/` directory exists with subdirectories and
/// no `projects/` directory exists yet.
pub fn migrate_to_project_hierarchy() -> Result<(), String> {
    let base = config_dir()?;
    let old_sessions_dir = base.join("sessions");
    let projects_dir = base.join("projects");

    // Only migrate if old layout exists
    if !old_sessions_dir.exists() {
        return Ok(());
    }

    // Check if old sessions dir has any actual session subdirectories
    let has_sessions = std::fs::read_dir(&old_sessions_dir)
        .map_err(|e| format!("Failed to read old sessions dir: {e}"))?
        .any(|e| e.map(|e| e.path().is_dir()).unwrap_or(false));

    if !has_sessions {
        // Empty sessions dir — just remove it
        let _ = std::fs::remove_dir(&old_sessions_dir);
        return Ok(());
    }

    // Already migrated if projects dir has content
    if projects_dir.exists() {
        let has_projects = std::fs::read_dir(&projects_dir)
            .map_err(|e| format!("Failed to read projects dir: {e}"))?
            .any(|e| e.map(|e| e.path().is_dir()).unwrap_or(false));
        if has_projects {
            return Ok(());
        }
    }

    eprintln!("Migrating flat sessions layout to project hierarchy...");

    // Load old projects.json if it exists
    let old_projects_file = base.join("projects.json");
    let old_projects: Vec<crate::models::ProjectEntry> = if old_projects_file.exists() {
        match std::fs::read_to_string(&old_projects_file) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    // Build a path→project lookup from old projects
    let mut path_to_project: std::collections::HashMap<String, crate::models::ProjectEntry> =
        old_projects
            .into_iter()
            .map(|p| (p.path.clone(), p))
            .collect();

    // Build new index
    let mut new_index = super::index::StorageIndex::default();

    // Scan old sessions
    let session_entries = std::fs::read_dir(&old_sessions_dir)
        .map_err(|e| format!("Failed to read old sessions dir: {e}"))?;

    for entry in session_entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
        let session_path = entry.path();

        if !session_path.is_dir() {
            continue;
        }

        let session_json = session_path.join("session.json");
        if !session_json.exists() {
            continue;
        }

        if let Err(e) = migrate_single_session(
            &session_path,
            &projects_dir,
            &mut path_to_project,
            &mut new_index,
        ) {
            let sid = session_path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
            eprintln!("Warning: Failed to migrate session {sid}: {e}");
        }
    }

    // Save the rebuilt index
    super::index::save(&new_index)?;
    super::index::invalidate_cache();

    // Clean up old files
    let _ = std::fs::remove_dir_all(&old_sessions_dir);
    let _ = std::fs::remove_file(&old_projects_file);
    let _ = std::fs::remove_file(base.join("run_index.json"));

    eprintln!("Migration complete: moved sessions into project hierarchy");

    Ok(())
}

/// Migrates a single session directory from old flat layout to project hierarchy.
fn migrate_single_session(
    session_path: &std::path::Path,
    projects_dir: &std::path::Path,
    path_to_project: &mut std::collections::HashMap<String, crate::models::ProjectEntry>,
    new_index: &mut super::index::StorageIndex,
) -> Result<(), String> {
    let session_json = session_path.join("session.json");
    let contents = std::fs::read_to_string(&session_json)
        .map_err(|e| format!("Failed to read session.json: {e}"))?;

    // Parse as raw JSON to extract project_path even without project_id
    let raw: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse session.json: {e}"))?;

    let project_path_str = raw
        .get("projectPath")
        .or_else(|| raw.get("project_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let session_id = session_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    if session_id.is_empty() {
        return Err("Empty session ID".to_string());
    }

    // Find or create project entry
    let project = if let Some(existing) = path_to_project.get(&project_path_str) {
        existing.clone()
    } else {
        let name = std::path::Path::new(&project_path_str)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path_str.clone());
        let entry = super::projects::create_project_entry(
            uuid::Uuid::new_v4().to_string(),
            project_path_str.clone(),
            name,
        );
        path_to_project.insert(project_path_str.clone(), entry.clone());
        entry
    };

    // Create project directory and write project.json
    let project_dir = projects_dir.join(&project.id);
    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create project dir: {e}"))?;

    let project_json_path = project_dir.join("project.json");
    if !project_json_path.exists() {
        let json = serde_json::to_string_pretty(&project)
            .map_err(|e| format!("Failed to serialise project: {e}"))?;
        super::atomic_write(&project_json_path, &json)?;
    }

    // Move session directory to new location
    let new_sessions_dir = project_dir.join("sessions");
    std::fs::create_dir_all(&new_sessions_dir)
        .map_err(|e| format!("Failed to create sessions dir: {e}"))?;

    let new_session_dir = new_sessions_dir.join(&session_id);
    std::fs::rename(session_path, &new_session_dir)
        .map_err(|e| format!("Failed to move session: {e}"))?;

    // Update session.json with project_id field
    let updated_json_path = new_session_dir.join("session.json");
    if let Ok(contents) = std::fs::read_to_string(&updated_json_path) {
        if let Ok(mut raw) = serde_json::from_str::<serde_json::Value>(&contents) {
            raw["projectId"] = serde_json::Value::String(project.id.clone());
            if let Ok(json) = serde_json::to_string_pretty(&raw) {
                let _ = super::atomic_write(&updated_json_path, &json);
            }
        }
    }

    // Add session to index
    new_index
        .sessions
        .insert(session_id.clone(), project.id.clone());

    // Add runs to index
    let runs_dir = new_session_dir.join("runs");
    if runs_dir.exists() {
        if let Ok(run_entries) = std::fs::read_dir(&runs_dir) {
            for run_entry in run_entries.flatten() {
                if run_entry.path().is_dir() {
                    if let Some(run_id) = run_entry.file_name().to_str() {
                        new_index
                            .runs
                            .insert(run_id.to_string(), session_id.clone());
                    }
                }
            }
        }
    }

    Ok(())
}
