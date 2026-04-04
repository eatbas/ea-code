use std::collections::HashMap;

use crate::models::ProjectEntry;

use super::{atomic_write, config_dir, now_rfc3339, with_projects_lock};

const PROJECTS_FILE: &str = "projects.json";

/// Load, mutate, and persist the projects list in a single locked operation.
fn with_project_store<T, F>(mutator: F) -> Result<T, String>
where
    F: FnOnce(&mut Vec<ProjectEntry>) -> Result<T, String>,
{
    with_projects_lock(|| {
        let file_path = config_dir()?.join(PROJECTS_FILE);
        let mut projects = if file_path.exists() {
            let contents = std::fs::read_to_string(&file_path)
                .map_err(|e| format!("Failed to read projects file: {e}"))?;
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            Vec::new()
        };
        let result = mutator(&mut projects)?;
        let json = serde_json::to_string_pretty(&projects)
            .map_err(|e| format!("Failed to serialise projects: {e}"))?;
        atomic_write(&file_path, &json)?;
        Ok(result)
    })
}

/// Reads all projects from projects.json.
pub fn list_projects(include_archived: bool) -> Result<Vec<ProjectEntry>, String> {
    with_projects_lock(|| {
        let path = config_dir()?.join(PROJECTS_FILE);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read projects file: {e}"))?;

        let projects: Vec<ProjectEntry> = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse projects file: {e}"))?;

        Ok(projects
            .into_iter()
            .filter(|project| include_archived || project.archived_at.is_none())
            .collect())
    })
}

/// Insert or update a project entry.
pub fn upsert(
    path: &str,
    name: &str,
    is_git_repo: bool,
    branch: Option<&str>,
) -> Result<(), String> {
    with_project_store(|projects| {
        let now = now_rfc3339();

        if let Some(existing) = projects.iter_mut().find(|p| p.path == path) {
            if existing.name.trim().is_empty() {
                existing.name = name.to_string();
            }
            existing.is_git_repo = is_git_repo;
            existing.branch = branch.map(|b| b.to_string());
            existing.last_opened = Some(now);
            existing.archived_at = None;
        } else {
            projects.push(ProjectEntry {
                id: uuid::Uuid::new_v4().to_string(),
                path: path.to_string(),
                name: name.to_string(),
                last_opened: Some(now.clone()),
                created_at: now,
                is_git_repo,
                branch: branch.map(|b| b.to_string()),
                archived_at: None,
            });
        }

        Ok(())
    })
}

/// Remove projects whose workspace folder no longer exists on disk.
pub fn cleanup_missing_projects() -> Result<(), String> {
    with_project_store(|projects| {
        projects.retain(|p| std::path::Path::new(&p.path).exists());
        Ok(())
    })
}

/// Delete a project entry by its path.
pub fn delete_project(project_path: &str) -> Result<(), String> {
    with_project_store(|projects| {
        projects.retain(|p| p.path != project_path);
        Ok(())
    })
}

pub fn rename_project(project_path: &str, name: &str) -> Result<ProjectEntry, String> {
    let trimmed = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        return Err("Project name must not be empty".to_string());
    }

    with_project_store(|projects| {
        let project = projects
            .iter_mut()
            .find(|p| p.path == project_path)
            .ok_or("Project not found")?;
        project.name = trimmed;
        project.archived_at = None;
        Ok(project.clone())
    })
}

pub fn archive_project(project_path: &str) -> Result<ProjectEntry, String> {
    with_project_store(|projects| {
        let project = projects
            .iter_mut()
            .find(|p| p.path == project_path)
            .ok_or("Project not found")?;
        if project.archived_at.is_none() {
            project.archived_at = Some(now_rfc3339());
        }
        Ok(project.clone())
    })
}

pub fn unarchive_project(project_path: &str) -> Result<ProjectEntry, String> {
    with_project_store(|projects| {
        let project = projects
            .iter_mut()
            .find(|p| p.path == project_path)
            .ok_or("Project not found")?;
        project.archived_at = None;
        Ok(project.clone())
    })
}

pub fn reorder_projects(ordered_project_paths: &[String]) -> Result<Vec<ProjectEntry>, String> {
    with_project_store(|projects| {
        if ordered_project_paths.len() != projects.len() {
            return Err("Project reorder payload does not match saved projects".to_string());
        }

        let mut remaining: HashMap<String, ProjectEntry> =
            projects.drain(..).map(|p| (p.path.clone(), p)).collect();

        for project_path in ordered_project_paths {
            let project = remaining
                .remove(project_path)
                .ok_or_else(|| format!("Project not found in reorder request: {project_path}"))?;
            projects.push(project);
        }

        if !remaining.is_empty() {
            return Err("Project reorder payload is missing saved projects".to_string());
        }

        Ok(projects.clone())
    })
}
