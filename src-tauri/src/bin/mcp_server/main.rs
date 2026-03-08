/// EA Code MCP Server — exposes session history and run data to agents.
///
/// Implements the Model Context Protocol (JSON-RPC 2.0 over stdio) so that
/// CLI agents (Claude, etc.) can query past runs, artefacts, and project
/// metadata during execution.
///
/// Usage: `ea-code-mcp --session-id <id>` or `ea-code-mcp` (without session).

mod args;
mod handlers;
mod json_rpc;
mod tools;

use std::io::{self, BufRead, Write};

use ea_code_lib::db;

fn main() {
    let cli_args = args::parse_args();

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

        let request: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = json_rpc::make_error(
                    &serde_json::Value::Null,
                    -32700,
                    &format!("Parse error: {e}"),
                );
                let _ = writeln!(
                    stdout_lock,
                    "{}",
                    serde_json::to_string(&err).unwrap_or_default()
                );
                let _ = stdout_lock.flush();
                continue;
            }
        };

        let response = json_rpc::handle_request(&pool, &request, &cli_args.session_id);

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
