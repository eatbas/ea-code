use crate::models::ProjectEntry;

use super::{atomic_write, config_dir, now_rfc3339, with_projects_lock};

const PROJECTS_FILE: &str = "projects.json";

/// Reads all projects from projects.json.
pub fn list_projects() -> Result<Vec<ProjectEntry>, String> {
    with_projects_lock(|| {
        let path = config_dir()?.join(PROJECTS_FILE);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read projects file: {e}"))?;

        let projects: Vec<ProjectEntry> = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse projects file: {e}"))?;

        Ok(projects)
    })
}

/// Insert or update a project entry.
pub fn upsert(
    path: &str,
    name: &str,
    is_git_repo: bool,
    branch: Option<&str>,
) -> Result<(), String> {
    with_projects_lock(|| {
        let file_path = config_dir()?.join(PROJECTS_FILE);
        let mut projects = if file_path.exists() {
            let contents = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read projects file: {e}"))?;
            serde_json::from_str::<Vec<ProjectEntry>>(&contents).unwrap_or_default()
        } else {
            Vec::new()
        };

        let now = now_rfc3339();

        if let Some(existing) = projects.iter_mut().find(|p| p.path == path) {
            existing.name = name.to_string();
            existing.is_git_repo = is_git_repo;
            existing.branch = branch.map(|b| b.to_string());
            existing.last_opened = Some(now);
        } else {
            projects.push(ProjectEntry {
                id: uuid::Uuid::new_v4().to_string(),
                path: path.to_string(),
                name: name.to_string(),
                last_opened: Some(now.clone()),
                created_at: now,
                is_git_repo,
                branch: branch.map(|b| b.to_string()),
            });
        }

        let json = serde_json::to_string_pretty(&projects)
            .map_err(|e| format!("Failed to serialise projects: {e}"))?;

        atomic_write(&file_path, &json)
    })
}

/// Remove projects whose workspace folder no longer exists on disk.
pub fn cleanup_missing_projects() -> Result<(), String> {
    with_projects_lock(|| {
        let file_path = config_dir()?.join(PROJECTS_FILE);

        if !file_path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read projects file: {e}"))?;

        let projects: Vec<ProjectEntry> = serde_json::from_str(&contents).unwrap_or_default();
        let before = projects.len();
        let filtered: Vec<ProjectEntry> = projects
            .into_iter()
            .filter(|p| std::path::Path::new(&p.path).exists())
            .collect();

        if filtered.len() < before {
            let json = serde_json::to_string_pretty(&filtered)
                .map_err(|e| format!("Failed to serialise projects: {e}"))?;
            atomic_write(&file_path, &json)?;
        }

        Ok(())
    })
}

/// Delete a project entry by its path.
pub fn delete_project(project_path: &str) -> Result<(), String> {
    with_projects_lock(|| {
        let file_path = config_dir()?.join(PROJECTS_FILE);

        if !file_path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read projects file: {e}"))?;

        let projects: Vec<ProjectEntry> = serde_json::from_str(&contents).unwrap_or_default();
        let filtered: Vec<ProjectEntry> = projects
            .into_iter()
            .filter(|p| p.path != project_path)
            .collect();

        let json = serde_json::to_string_pretty(&filtered)
            .map_err(|e| format!("Failed to serialise projects: {e}"))?;

        atomic_write(&file_path, &json)
    })
}
