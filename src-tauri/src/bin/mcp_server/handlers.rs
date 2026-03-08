/// MCP tool handler implementations — each function processes a tool call
/// and returns structured JSON or an error string.

use serde_json::{json, Value};

use diesel::prelude::*;
use ea_code_lib::db::{self, DbPool};
use ea_code_lib::schema::{projects, runs, sessions};

/// Handles the `get_session_history` tool: returns recent runs for a session.
pub fn handle_get_session_history(
    pool: &DbPool,
    args: &Value,
    default_session: &Option<String>,
) -> Result<Value, String> {
    let session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| default_session.clone())
        .ok_or_else(|| "No session_id provided and no default session set.".to_string())?;

    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

    // Get session info
    let session = db::sessions::get_by_id(pool, &session_id)?
        .ok_or_else(|| format!("Session {session_id} not found."))?;

    // Get runs for this session
    let all_runs = db::runs::list_for_session(pool, &session_id)?;
    let runs_to_show: Vec<_> = all_runs.into_iter().take(limit as usize).collect();

    Ok(json!({
        "sessionId": session.id,
        "title": session.title,
        "runCount": runs_to_show.len(),
        "runs": runs_to_show,
    }))
}

/// Handles the `search_runs` tool: searches runs by prompt text and optional
/// workspace path filter.
pub fn handle_search_runs(pool: &DbPool, args: &Value) -> Result<Value, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: query".to_string())?;

    let workspace_path = args.get("workspace_path").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

    let mut conn = db::get_conn(pool)?;

    let mut query_builder = runs::table
        .inner_join(sessions::table.on(sessions::id.eq(runs::session_id)))
        .inner_join(projects::table.on(projects::id.eq(sessions::project_id)))
        .filter(runs::prompt.like(format!("%{query}%")))
        .order(runs::started_at.desc())
        .limit(limit)
        .select((
            runs::id,
            runs::prompt,
            runs::status,
            runs::final_verdict,
            runs::executive_summary,
            runs::started_at,
            runs::completed_at,
            projects::path,
        ))
        .into_boxed();

    if let Some(wp) = workspace_path {
        query_builder = query_builder.filter(projects::path.eq(wp));
    }

    let results: Vec<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        String,
    )> = query_builder
        .load(&mut conn)
        .map_err(|e| format!("Search query failed: {e}"))?;

    let runs_json: Vec<Value> = results
        .into_iter()
        .map(
            |(id, prompt, status, verdict, executive_summary, started, completed, proj_path)| {
                json!({
                    "id": id,
                    "prompt": prompt,
                    "status": status,
                    "finalVerdict": verdict,
                    "executiveSummary": executive_summary,
                    "startedAt": started,
                    "completedAt": completed,
                    "projectPath": proj_path,
                })
            },
        )
        .collect();

    Ok(json!({ "results": runs_json }))
}

/// Handles the `get_run_output` tool: returns full run detail and artefacts.
pub fn handle_get_run_output(pool: &DbPool, args: &Value) -> Result<Value, String> {
    let run_id = args
        .get("run_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: run_id".to_string())?;

    let detail = db::runs::get_full(pool, run_id)?;
    let artifacts = db::artifacts::get_for_run(pool, run_id)?;

    Ok(json!({
        "run": detail,
        "artifacts": artifacts,
    }))
}

/// Handles the `get_project_summary` tool: returns project metadata and run
/// statistics.
pub fn handle_get_project_summary(pool: &DbPool, args: &Value) -> Result<Value, String> {
    let workspace_path = args
        .get("workspace_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: workspace_path".to_string())?;

    let project = db::projects::get_by_path(pool, workspace_path)?
        .ok_or_else(|| format!("Project not found for path: {workspace_path}"))?;

    let all_sessions = db::sessions::list_for_project(pool, project.id, 20)?;

    // Count total runs across all sessions
    let total_runs: i64 = all_sessions.iter().map(|s| s.run_count).sum();

    Ok(json!({
        "project": {
            "id": project.id,
            "path": project.path,
            "name": project.name,
            "isGitRepo": project.is_git_repo,
            "branch": project.branch,
            "lastOpened": project.last_opened,
        },
        "sessionCount": all_sessions.len(),
        "totalRuns": total_runs,
        "recentSessions": all_sessions,
    }))
}
