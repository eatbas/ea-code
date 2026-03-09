use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::{projects, runs, sessions};

use super::models::{NewSession, RunRow, SessionRow, SessionSummary};

/// Creates a new session (conversation thread) for a project.
pub fn create(pool: &DbPool, id: &str, project_id: i32, title: &str) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::insert_into(sessions::table)
        .values(&NewSession {
            id,
            project_id,
            title,
        })
        .execute(&mut conn)
        .map_err(|e| format!("Failed to create session: {e}"))?;

    Ok(())
}

/// Returns sessions for a given project, ordered by last update (most recent first).
/// Each summary includes run count and last prompt/status.
pub fn list_for_project(
    pool: &DbPool,
    project_id: i32,
    limit: i64,
) -> Result<Vec<SessionSummary>, String> {
    let mut conn = super::get_conn(pool)?;

    let session_rows: Vec<SessionRow> = sessions::table
        .filter(sessions::project_id.eq(project_id))
        .order(sessions::updated_at.desc())
        .limit(limit)
        .load(&mut conn)
        .map_err(|e| format!("Failed to list sessions: {e}"))?;

    let mut summaries = Vec::with_capacity(session_rows.len());
    for s in session_rows {
        // Count runs in this session
        let run_count: i64 = runs::table
            .filter(runs::session_id.eq(&s.id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| format!("Failed to count runs: {e}"))?;

        // Get the most recent run for preview
        let last_run: Option<RunRow> = runs::table
            .filter(runs::session_id.eq(&s.id))
            .order(runs::started_at.desc())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to get last run: {e}"))?;

        summaries.push(SessionSummary {
            id: s.id,
            title: s.title,
            project_id: s.project_id,
            run_count,
            last_prompt: last_run.as_ref().map(|r| truncate(&r.prompt, 80)),
            last_status: last_run.map(|r| r.status),
            created_at: s.created_at,
            updated_at: s.updated_at,
        });
    }

    Ok(summaries)
}

/// Retrieves a session by ID, including the associated project path.
pub fn get_by_id(pool: &DbPool, session_id: &str) -> Result<Option<SessionRow>, String> {
    let mut conn = super::get_conn(pool)?;

    sessions::table
        .find(session_id)
        .first(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to get session: {e}"))
}

/// Returns the project path for a given session.
pub fn get_project_path(pool: &DbPool, session_id: &str) -> Result<String, String> {
    let mut conn = super::get_conn(pool)?;

    let project_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::project_id)
        .first(&mut conn)
        .map_err(|e| format!("Session not found: {e}"))?;

    projects::table
        .find(project_id)
        .select(projects::path)
        .first(&mut conn)
        .map_err(|e| format!("Project not found: {e}"))
}

/// Updates the session's `updated_at` timestamp (called after a run completes).
pub fn touch(pool: &DbPool, session_id: &str) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let now = super::now_rfc3339();

    diesel::update(sessions::table.find(session_id))
        .set(sessions::updated_at.eq(&now))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to touch session: {e}"))?;

    Ok(())
}

/// Updates the session title.
pub fn update_title(pool: &DbPool, session_id: &str, title: &str) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::update(sessions::table.find(session_id))
        .set(sessions::title.eq(title))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update session title: {e}"))?;

    Ok(())
}

/// Deletes a session and all associated data (cascaded).
pub fn delete(pool: &DbPool, session_id: &str) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    diesel::delete(sessions::table.find(session_id))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete session: {e}"))?;

    Ok(())
}

/// Truncates a string to the given character count, appending "..." if truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}
