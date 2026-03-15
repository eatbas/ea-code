use crate::models::ProjectEntry;

use super::{atomic_write, config_dir, now_rfc3339, validate_id, with_projects_lock};

const MAX_PROJECTS: usize = 50;

/// Returns the projects root directory.
fn projects_dir() -> Result<std::path::PathBuf, String> {
    Ok(config_dir()?.join("projects"))
}

/// Returns the project directory for a given project ID.
pub fn project_dir(id: &str) -> Result<std::path::PathBuf, String> {
    validate_id(id)?;
    Ok(projects_dir()?.join(id))
}

/// Returns the project.json file path for a project.
fn project_path(id: &str) -> Result<std::path::PathBuf, String> {
    Ok(project_dir(id)?.join("project.json"))
}

/// Sorts projects by last_opened descending (most recent first).
/// Falls back to created_at if last_opened is not set.
fn sort_by_last_opened(projects: &mut [ProjectEntry]) {
    projects.sort_by(|a, b| {
        b.last_opened
            .as_ref()
            .unwrap_or(&b.created_at)
            .cmp(a.last_opened.as_ref().unwrap_or(&a.created_at))
    });
}

/// Reads all projects by scanning `projects/*/project.json`, sorted by last_opened descending.
/// Returns empty vector if no projects exist.
pub fn read_projects() -> Result<Vec<ProjectEntry>, String> {
    let dir = projects_dir()?;

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("Failed to read projects directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_dir() {
            let json_path = path.join("project.json");
            if json_path.exists() {
                match std::fs::read_to_string(&json_path) {
                    Ok(contents) => match serde_json::from_str::<ProjectEntry>(&contents) {
                        Ok(project) => projects.push(project),
                        Err(e) => eprintln!(
                            "Warning: Failed to parse project file {}: {e}",
                            json_path.display()
                        ),
                    },
                    Err(e) => eprintln!(
                        "Warning: Failed to read project file {}: {e}",
                        json_path.display()
                    ),
                }
            }
        }
    }

    sort_by_last_opened(&mut projects);

    Ok(projects)
}

/// Finds a project by its filesystem path.
/// Returns `None` if no project matches.
pub fn find_by_path(path: &str) -> Option<ProjectEntry> {
    let projects = read_projects().ok()?;
    projects.into_iter().find(|p| p.path == path)
}

/// Writes a single project's metadata to `projects/{id}/project.json`.
fn write_project(entry: &ProjectEntry) -> Result<(), String> {
    let json = serde_json::to_string_pretty(entry)
        .map_err(|e| format!("Failed to serialise project: {e}"))?;
    let path = project_path(&entry.id)?;
    atomic_write(&path, &json)
}

/// Adds or updates a project entry, maintaining the 50-entry cap.
/// If the project already exists (by path), updates last_opened.
/// H8: Protected by file lock for concurrent access.
pub fn add_project(entry: &ProjectEntry) -> Result<(), String> {
    with_projects_lock(|| {
        let mut projects = read_projects()?;

        // Check if project already exists by path
        if let Some(existing) = projects.iter_mut().find(|p| p.path == entry.path) {
            // Update existing entry
            existing.last_opened = Some(now_rfc3339());
            existing.name = entry.name.clone();
            write_project(existing)?;
        } else {
            // Add new entry
            let mut new_entry = entry.clone();
            new_entry.last_opened = Some(now_rfc3339());
            write_project(&new_entry)?;
            projects.push(new_entry);
        }

        // Enforce cap: remove oldest projects beyond MAX_PROJECTS
        sort_by_last_opened(&mut projects);
        if projects.len() > MAX_PROJECTS {
            for old in projects.drain(MAX_PROJECTS..) {
                let dir = project_dir(&old.id)?;
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(&dir);
                }
            }
        }

        Ok(())
    })
}

/// Removes a project by its path, deleting the entire project directory.
/// H8: Protected by file lock for concurrent access.
pub fn remove_project(path: &str) -> Result<(), String> {
    with_projects_lock(|| {
        let projects = read_projects()?;
        let entry = projects
            .iter()
            .find(|p| p.path == path)
            .ok_or_else(|| format!("Project not found: {path}"))?;

        let dir = project_dir(&entry.id)?;
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .map_err(|e| format!("Failed to delete project directory: {e}"))?;
        }

        Ok(())
    })
}

/// Creates a new project entry with the current timestamp.
pub fn create_project_entry(id: String, path: String, name: String) -> ProjectEntry {
    let now = now_rfc3339();
    ProjectEntry {
        id,
        path,
        name,
        last_opened: Some(now.clone()),
        created_at: now,
        is_git_repo: false,
        branch: None,
    }
}

/// Upserts a project entry from workspace selection.
/// Creates new entry if path doesn't exist, updates last_opened if it does.
/// H8: Protected by file lock for concurrent access.
pub fn upsert(
    path: &str,
    name: &str,
    is_git_repo: bool,
    branch: Option<&str>,
) -> Result<(), String> {
    with_projects_lock(|| {
        let mut projects = read_projects()?;

        if let Some(existing) = projects.iter_mut().find(|p| p.path == path) {
            // Update existing entry
            existing.last_opened = Some(now_rfc3339());
            existing.name = name.to_string();
            existing.is_git_repo = is_git_repo;
            existing.branch = branch.map(|b| b.to_string());
            write_project(existing)?;
        } else {
            // Create new entry
            let id = uuid::Uuid::new_v4().to_string();
            let now = now_rfc3339();
            let entry = ProjectEntry {
                id,
                path: path.to_string(),
                name: name.to_string(),
                last_opened: Some(now.clone()),
                created_at: now,
                is_git_repo,
                branch: branch.map(|b| b.to_string()),
            };
            write_project(&entry)?;
            projects.push(entry);
        }

        // Enforce cap
        sort_by_last_opened(&mut projects);
        if projects.len() > MAX_PROJECTS {
            for old in projects.drain(MAX_PROJECTS..) {
                let dir = project_dir(&old.id)?;
                if dir.exists() {
                    let _ = std::fs::remove_dir_all(&dir);
                }
            }
        }

        Ok(())
    })
}
