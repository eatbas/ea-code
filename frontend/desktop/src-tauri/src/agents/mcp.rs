use std::collections::BTreeMap;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::DbPool;

fn locate_history_mcp_binary() -> Option<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("ea-code-mcp");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    let lookup_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(output) = std::process::Command::new(lookup_cmd)
        .arg("ea-code-mcp")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

fn resolve_server_command(command: &str) -> Option<String> {
    if command == "ea-code-mcp" {
        return locate_history_mcp_binary();
    }
    Some(command.to_string())
}

/// Builds a temporary MCP config file for a CLI from active DB-backed servers.
pub fn build_mcp_config_for_cli(
    db: &DbPool,
    cli_name: &str,
    session_id: Option<&str>,
) -> Option<String> {
    let active = match crate::db::mcp::get_active_servers_for_cli(db, cli_name) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("MCP config lookup failed for {cli_name}: {e}");
            return None;
        }
    };
    if active.is_empty() {
        return None;
    }

    let mut servers = BTreeMap::<String, serde_json::Value>::new();
    for server in active {
        let command = match resolve_server_command(&server.command) {
            Some(cmd) => cmd,
            None => {
                eprintln!(
                    "Skipping MCP server {} because command could not be resolved",
                    server.id
                );
                continue;
            }
        };

        let mut args = server.args.clone();
        if server.id == "ea-code-history" {
            if let Some(sid) = session_id {
                args.push("--session-id".to_string());
                args.push(sid.to_string());
            }
        }

        let mut payload = serde_json::Map::new();
        payload.insert("command".to_string(), serde_json::Value::String(command));
        payload.insert(
            "args".to_string(),
            serde_json::Value::Array(
                args.into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
        if !server.env.is_empty() {
            let env = serde_json::to_value(server.env).ok()?;
            payload.insert("env".to_string(), env);
        }
        servers.insert(server.id.clone(), serde_json::Value::Object(payload));
    }

    if servers.is_empty() {
        return None;
    }

    let config = serde_json::json!({
        "mcpServers": servers
    });

    let config_dir = dirs::config_dir()?.join("ea-code");
    std::fs::create_dir_all(&config_dir).ok()?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let config_path = config_dir.join(format!("mcp-config-{cli_name}-{stamp}.json"));
    let mut file = std::fs::File::create(&config_path).ok()?;
    file.write_all(config.to_string().as_bytes()).ok()?;
    Some(config_path.to_string_lossy().to_string())
}
