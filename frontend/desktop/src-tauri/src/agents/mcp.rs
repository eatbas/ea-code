use std::collections::BTreeMap;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::DbPool;

/// Builds a temporary MCP config file for a CLI from active DB-backed servers.
pub fn build_mcp_config_for_cli(
    db: &DbPool,
    cli_name: &str,
    _session_id: Option<&str>,
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
        let command = server.command.clone();
        let args = server.args.clone();

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
