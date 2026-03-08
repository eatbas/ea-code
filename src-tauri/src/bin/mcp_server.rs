/// EA Code MCP Server — exposes session history and run data to agents.
///
/// Implements the Model Context Protocol (JSON-RPC 2.0 over stdio) so that
/// CLI agents (Claude, etc.) can query past runs, artefacts, and project
/// metadata during execution.
///
/// Usage: `ea-code-mcp --session-id <id>` or `ea-code-mcp` (without session).
use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use ea_code_lib::db::{self, DbPool};

// ── Argument parsing ────────────────────────────────────────────────────

struct Args {
    session_id: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut session_id = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--session-id" && i + 1 < args.len() {
            session_id = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    Args { session_id }
}

// ── Tool definitions ────────────────────────────────────────────────────

fn tool_definitions() -> Value {
    json!([
        {
            "name": "get_session_history",
            "description": "Returns previous runs in the current session, including prompts, verdicts, and errors. Use this to understand what has already been attempted in this conversation thread.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to query. Defaults to the current session if omitted."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of runs to return (default: 10)."
                    }
                }
            }
        },
        {
            "name": "search_runs",
            "description": "Search past pipeline runs by prompt text or workspace path. Useful for finding previous attempts at similar tasks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for in run prompts."
                    },
                    "workspace_path": {
                        "type": "string",
                        "description": "Filter by workspace path."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 10)."
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "get_run_output",
            "description": "Returns full output and artefacts (diffs, reviews, judge verdicts) for a specific run. Use a run ID obtained from get_session_history or search_runs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "run_id": {
                        "type": "string",
                        "description": "The run ID to retrieve."
                    }
                },
                "required": ["run_id"]
            }
        },
        {
            "name": "get_project_summary",
            "description": "Returns project info and recent run statistics for a workspace path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace_path": {
                        "type": "string",
                        "description": "Filesystem path of the project workspace."
                    }
                },
                "required": ["workspace_path"]
            }
        }
    ])
}

// ── Tool handlers ───────────────────────────────────────────────────────

fn handle_get_session_history(
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
    let runs = db::runs::list_for_session(pool, &session_id)?;
    let runs_to_show: Vec<_> = runs.into_iter().take(limit as usize).collect();

    Ok(json!({
        "sessionId": session.id,
        "title": session.title,
        "runCount": runs_to_show.len(),
        "runs": runs_to_show,
    }))
}

fn handle_search_runs(pool: &DbPool, args: &Value) -> Result<Value, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: query".to_string())?;

    let workspace_path = args.get("workspace_path").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);

    // Search runs by prompt text, optionally filtered by workspace/project
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    use diesel::prelude::*;
    use ea_code_lib::schema::{projects, runs, sessions};

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

fn handle_get_run_output(pool: &DbPool, args: &Value) -> Result<Value, String> {
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

fn handle_get_project_summary(pool: &DbPool, args: &Value) -> Result<Value, String> {
    let workspace_path = args
        .get("workspace_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter: workspace_path".to_string())?;

    let project = db::projects::get_by_path(pool, workspace_path)?
        .ok_or_else(|| format!("Project not found for path: {workspace_path}"))?;

    let sessions = db::sessions::list_for_project(pool, project.id, 20)?;

    // Count total runs across all sessions
    let total_runs: i64 = sessions.iter().map(|s| s.run_count).sum();

    Ok(json!({
        "project": {
            "id": project.id,
            "path": project.path,
            "name": project.name,
            "isGitRepo": project.is_git_repo,
            "branch": project.branch,
            "lastOpened": project.last_opened,
        },
        "sessionCount": sessions.len(),
        "totalRuns": total_runs,
        "recentSessions": sessions,
    }))
}

// ── JSON-RPC plumbing ───────────────────────────────────────────────────

fn make_response(id: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn make_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

fn make_tool_result(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn handle_request(pool: &DbPool, request: &Value, default_session: &Option<String>) -> Value {
    let id = request.get("id").unwrap_or(&Value::Null);
    let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");

    match method {
        "initialize" => make_response(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ea-code-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        ),

        "notifications/initialized" => {
            // Notification — no response needed
            return Value::Null;
        }

        "tools/list" => make_response(id, json!({ "tools": tool_definitions() })),

        "tools/call" => {
            let params = request.get("params").unwrap_or(&Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let empty_args = json!({});
            let arguments = params.get("arguments").unwrap_or(&empty_args);

            let result = match tool_name {
                "get_session_history" => {
                    handle_get_session_history(pool, arguments, default_session)
                }
                "search_runs" => handle_search_runs(pool, arguments),
                "get_run_output" => handle_get_run_output(pool, arguments),
                "get_project_summary" => handle_get_project_summary(pool, arguments),
                _ => Err(format!("Unknown tool: {tool_name}")),
            };

            match result {
                Ok(data) => {
                    let text = serde_json::to_string_pretty(&data).unwrap_or_default();
                    make_response(id, make_tool_result(&text, false))
                }
                Err(e) => make_response(id, make_tool_result(&e, true)),
            }
        }

        // Ignore unknown notifications (no id = notification)
        _ if id.is_null() => Value::Null,

        _ => make_error(id, -32601, &format!("Method not found: {method}")),
    }
}

// ── Entry point ─────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    // Initialise the database pool (read-only access to existing DB)
    let pool = match db::init_db() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ea-code-mcp: failed to initialise database: {e}");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    // Process JSON-RPC messages line by line
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = make_error(&Value::Null, -32700, &format!("Parse error: {e}"));
                let _ = writeln!(
                    stdout_lock,
                    "{}",
                    serde_json::to_string(&err).unwrap_or_default()
                );
                let _ = stdout_lock.flush();
                continue;
            }
        };

        let response = handle_request(&pool, &request, &args.session_id);

        // Null response means no reply needed (notification)
        if response.is_null() {
            continue;
        }

        let _ = writeln!(
            stdout_lock,
            "{}",
            serde_json::to_string(&response).unwrap_or_default()
        );
        let _ = stdout_lock.flush();
    }
}
