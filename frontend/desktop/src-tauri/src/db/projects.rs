use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::projects;

use super::models::{NewProject, ProjectRow};

/// Inserts a project or updates `last_opened` if it already exists.
/// Returns the project ID.
pub fn upsert(
    pool: &DbPool,
    path: &str,
    name: &str,
    is_git_repo: bool,
    branch: Option<&str>,
) -> Result<i32, String> {
    let mut conn = super::get_conn(pool)?;

    let now = super::now_rfc3339();

    // Try to find an existing project by path
    let existing: Option<ProjectRow> = projects::table
        .filter(projects::path.eq(path))
        .first(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to query project: {e}"))?;

    if let Some(row) = existing {
        // Update last_opened, branch, and git status
        diesel::update(projects::table.find(row.id))
            .set((
                projects::last_opened.eq(&now),
                projects::is_git_repo.eq(is_git_repo),
                projects::branch.eq(branch),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Failed to update project: {e}"))?;

        return Ok(row.id);
    }

    // Insert new project
    diesel::insert_into(projects::table)
        .values(&NewProject {
            path,
            name,
            is_git_repo,
            branch,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to insert project: {e}"))?;

    // Retrieve the auto-generated ID
    projects::table
        .filter(projects::path.eq(path))
        .select(projects::id)
        .first::<i32>(&mut conn)
        .map_err(|e| format!("Failed to retrieve project id: {e}"))
}

/// Returns recent projects ordered by last opened (descending).
pub fn list_recent(pool: &DbPool, limit: i64) -> Result<Vec<ProjectRow>, String> {
    let mut conn = super::get_conn(pool)?;

    projects::table
        .order(projects::last_opened.desc())
        .limit(limit)
        .load::<ProjectRow>(&mut conn)
        .map_err(|e| format!("Failed to list projects: {e}"))
}

/// Finds a project by its filesystem path. Returns `None` if not found.
pub fn get_by_path(pool: &DbPool, path: &str) -> Result<Option<ProjectRow>, String> {
    let mut conn = super::get_conn(pool)?;

    projects::table
        .filter(projects::path.eq(path))
        .first::<ProjectRow>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to query project: {e}"))
}
