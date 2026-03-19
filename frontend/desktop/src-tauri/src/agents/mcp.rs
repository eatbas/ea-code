use std::collections::BTreeMap;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::storage;

/// Builds a temporary MCP config file for a CLI from active file-backed servers.
#[allow(dead_code)]
pub fn build_mcp_config_for_cli(cli_name: &str, _session_id: Option<&str>) -> Option<String> {
    let mcp_config = storage::mcp::read_mcp_config().ok()?;

    // Filter to servers that are enabled and bound to this CLI
    let active: Vec<(String, crate::models::McpServerConfig)> = mcp_config
        .servers
        .into_iter()
        .filter(|(id, _)| {
            // Check if this server is bound to the CLI
            mcp_config
                .cli_bindings
                .get(cli_name)
                .map(|bindings| bindings.contains(id))
                .unwrap_or(false)
        })
        .collect();

    if active.is_empty() {
        return None;
    }

    let mut servers = BTreeMap::<String, serde_json::Value>::new();
    for (server_id, server) in active {
        let mut payload = serde_json::Map::new();
        payload.insert(
            "command".to_string(),
            serde_json::Value::String(server.command),
        );
        payload.insert(
            "args".to_string(),
            serde_json::Value::Array(
                server
                    .args
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
        if !server.env.is_empty() {
            let env = serde_json::to_value(&server.env).ok()?;
            payload.insert("env".to_string(), env);
        }
        servers.insert(server_id, serde_json::Value::Object(payload));
    }

    if servers.is_empty() {
        return None;
    }

    let config = serde_json::json!({
        "mcpServers": servers
    });

    let config_dir = crate::storage::config_dir().ok()?;
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
