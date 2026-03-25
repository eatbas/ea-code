use crate::models::ProjectEntry;

use super::{atomic_write, config_dir, now_rfc3339, with_projects_lock};

const MAX_PROJECTS: usize = 50;

/// Returns the path to the flat projects registry: `~/.ea-code/projects.json`.
fn projects_file() -> Result<std::path::PathBuf, String> {
    Ok(config_dir()?.join("projects.json"))
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

/// Reads the flat projects.json array, sorted by last_opened descending.
/// Returns empty vector if file does not exist.
pub fn read_projects() -> Result<Vec<ProjectEntry>, String> {
    let path = projects_file()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read projects.json: {e}"))?;

    let mut projects: Vec<ProjectEntry> =
        serde_json::from_str(&contents).unwrap_or_default();

    sort_by_last_opened(&mut projects);

    Ok(projects)
}

/// Writes the full projects array to disk atomically.
fn write_projects(projects: &[ProjectEntry]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(projects)
        .map_err(|e| format!("Failed to serialise projects: {e}"))?;
    let path = projects_file()?;
    atomic_write(&path, &json)
}

/// Finds a project by its filesystem path.
/// Returns `None` if no project matches.
pub fn find_by_path(path: &str) -> Option<ProjectEntry> {
    let projects = read_projects().ok()?;
    projects.into_iter().find(|p| p.path == path)
}

/// Adds or updates a project entry, maintaining the 50-entry cap.
/// If the project already exists (by path), updates last_opened.
/// Also ensures the workspace `.ea-code/` directory exists.
/// H8: Protected by file lock for concurrent access.
pub fn add_project(entry: &ProjectEntry) -> Result<(), String> {
    with_projects_lock(|| {
        let mut projects = read_projects()?;

        if let Some(existing) = projects.iter_mut().find(|p| p.path == entry.path) {
            existing.last_opened = Some(now_rfc3339());
            existing.name = entry.name.clone();
        } else {
            let mut new_entry = entry.clone();
            new_entry.last_opened = Some(now_rfc3339());
            projects.push(new_entry);
        }

        // Enforce cap: remove oldest projects beyond MAX_PROJECTS
        sort_by_last_opened(&mut projects);
        if projects.len() > MAX_PROJECTS {
            projects.truncate(MAX_PROJECTS);
        }

        write_projects(&projects)?;

        // Ensure workspace data directory exists
        let _ = super::ensure_workspace_dirs(&entry.path);

        Ok(())
    })
}

/// Removes a project by its path from the flat registry.
/// H8: Protected by file lock for concurrent access.
pub fn remove_project(path: &str) -> Result<(), String> {
    with_projects_lock(|| {
        let mut projects = read_projects()?;
        let before = projects.len();
        projects.retain(|p| p.path != path);

        if projects.len() == before {
            return Err(format!("Project not found: {path}"));
        }

        write_projects(&projects)
    })
}

/// Removes projects whose filesystem path no longer exists on disk.
/// Called during startup to keep the project list clean.
pub fn cleanup_missing_projects() -> Result<(), String> {
    let projects = read_projects()?;
    let mut changed = false;
    let mut kept = Vec::new();

    for project in projects {
        let p = std::path::Path::new(&project.path);
        if p.exists() {
            kept.push(project);
        } else {
            eprintln!(
                "Removing stale project (folder missing): {} → {}",
                project.name, project.path
            );
            changed = true;
        }
    }

    if changed {
        write_projects(&kept)?;
    }
    Ok(())
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
/// Also ensures the workspace `.ea-code/` directory exists.
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
            existing.last_opened = Some(now_rfc3339());
            existing.name = name.to_string();
            existing.is_git_repo = is_git_repo;
            existing.branch = branch.map(|b| b.to_string());
        } else {
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
            projects.push(entry);
        }

        // Enforce cap
        sort_by_last_opened(&mut projects);
        if projects.len() > MAX_PROJECTS {
            projects.truncate(MAX_PROJECTS);
        }

        write_projects(&projects)?;

        // Ensure workspace data directory exists
        let _ = super::ensure_workspace_dirs(path);

        Ok(())
    })
}
