use std::collections::HashMap;

use crate::models::McpServer;
use crate::models::{McpConfigFile, McpServerConfig};
use crate::storage;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CreateMcpServerPayload {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub is_enabled: Option<bool>,
    pub cli_bindings: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UpdateMcpServerPayload {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub args: Option<String>,
    pub env: Option<String>,
    pub is_enabled: Option<bool>,
    pub cli_bindings: Option<Vec<String>>,
}

fn normalise_id(raw: Option<&str>, fallback_name: &str) -> String {
    if let Some(id) = raw.map(str::trim).filter(|s| !s.is_empty()) {
        return id.to_lowercase();
    }
    let slug = fallback_name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if slug.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        slug
    }
}

fn normalise_json_array(raw: Option<&str>) -> Result<String, String> {
    let text = raw.unwrap_or("[]").trim();
    let parsed = serde_json::from_str::<Vec<String>>(text)
        .map_err(|e| format!("Invalid MCP args JSON array: {e}"))?;
    serde_json::to_string(&parsed).map_err(|e| format!("Failed to serialise args: {e}"))
}

fn normalise_json_object(raw: Option<&str>) -> Result<String, String> {
    let text = raw.unwrap_or("{}").trim();
    let parsed = serde_json::from_str::<HashMap<String, String>>(text)
        .map_err(|e| format!("Invalid MCP env JSON object: {e}"))?;
    serde_json::to_string(&parsed).map_err(|e| format!("Failed to serialise env: {e}"))
}

fn to_model(
    id: &str,
    name: &str,
    description: &str,
    config: &McpServerConfig,
    cli_bindings: Vec<String>,
) -> McpServer {
    McpServer {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        command: config.command.clone(),
        args: serde_json::to_string(&config.args).unwrap_or_else(|_| "[]".to_string()),
        env: serde_json::to_string(&config.env).unwrap_or_else(|_| "{}".to_string()),
        is_enabled: true, // File-based storage doesn't track enabled state separately
        is_builtin: false, // Custom servers are never built-in
        cli_bindings,
        created_at: storage::now_rfc3339(),
        updated_at: storage::now_rfc3339(),
    }
}

#[tauri::command]
pub async fn list_mcp_servers() -> Result<Vec<McpServer>, String> {
    let config = storage::mcp::read_mcp_config()?;

    let mut servers = Vec::new();
    for (id, server_config) in &config.servers {
        let cli_bindings = config
            .cli_bindings
            .iter()
            .filter_map(|(cli, server_ids)| {
                if server_ids.contains(id) {
                    Some(cli.clone())
                } else {
                    None
                }
            })
            .collect();

        servers.push(to_model(id, id, "", server_config, cli_bindings));
    }

    Ok(servers)
}

#[tauri::command]
pub async fn list_mcp_capable_clis() -> Result<Vec<String>, String> {
    let settings = storage::settings::read_settings()?;
    let mut available = Vec::new();

    for cli_name in crate::models::AI_CLI_NAMES {
        if let Some(_path) = settings.path_for_cli(cli_name) {
            // For file-based storage, we just check if the CLI is configured
            // The actual availability check happens at runtime
            available.push(cli_name.to_string());
        }
    }

    Ok(available)
}

#[tauri::command]
pub async fn set_mcp_server_enabled(_server_id: String, _enabled: bool) -> Result<(), String> {
    // File-based storage doesn't track enabled state separately
    // Servers are always "enabled" if they exist
    Ok(())
}

#[tauri::command]
pub async fn set_mcp_server_bindings(
    server_id: String,
    cli_names: Vec<String>,
) -> Result<(), String> {
    storage::mcp::set_server_bindings(server_id.trim(), &cli_names)
}

#[tauri::command]
pub async fn create_mcp_server(payload: CreateMcpServerPayload) -> Result<McpServer, String> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err("MCP server name is required".to_string());
    }
    let command = payload.command.trim().to_string();
    if command.is_empty() {
        return Err("MCP server command is required".to_string());
    }

    let id = normalise_id(payload.id.as_deref(), &name);
    let description = payload.description.unwrap_or_default().trim().to_string();
    let args_json = normalise_json_array(payload.args.as_deref())?;
    let env_json = normalise_json_object(payload.env.as_deref())?;

    let args: Vec<String> =
        serde_json::from_str(&args_json).map_err(|e| format!("Failed to parse args: {e}"))?;
    let env: HashMap<String, String> =
        serde_json::from_str(&env_json).map_err(|e| format!("Failed to parse env: {e}"))?;

    let config = McpServerConfig { command, args, env };
    storage::mcp::add_server(&id, &config)?;

    let bindings = payload.cli_bindings.unwrap_or_default();
    if !bindings.is_empty() {
        storage::mcp::set_server_bindings(&id, &bindings)?;
    }

    Ok(to_model(&id, &name, &description, &config, bindings))
}

#[tauri::command]
pub async fn update_mcp_server(payload: UpdateMcpServerPayload) -> Result<McpServer, String> {
    let id = payload.id.trim().to_string();
    if id.is_empty() {
        return Err("MCP server id is required".to_string());
    }
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err("MCP server name is required".to_string());
    }
    let command = payload.command.trim().to_string();
    if command.is_empty() {
        return Err("MCP server command is required".to_string());
    }

    let description = payload.description.unwrap_or_default().trim().to_string();
    let args_json = normalise_json_array(payload.args.as_deref())?;
    let env_json = normalise_json_object(payload.env.as_deref())?;

    let args: Vec<String> =
        serde_json::from_str(&args_json).map_err(|e| format!("Failed to parse args: {e}"))?;
    let env: HashMap<String, String> =
        serde_json::from_str(&env_json).map_err(|e| format!("Failed to parse env: {e}"))?;

    let config = McpServerConfig { command, args, env };

    // Remove and re-add to update
    let _ = storage::mcp::remove_server(&id);
    storage::mcp::add_server(&id, &config)?;

    let bindings = payload.cli_bindings.unwrap_or_default();
    if !bindings.is_empty() {
        storage::mcp::set_server_bindings(&id, &bindings)?;
    }

    Ok(to_model(&id, &name, &description, &config, bindings))
}

#[tauri::command]
pub async fn delete_mcp_server(server_id: String) -> Result<(), String> {
    storage::mcp::remove_server(server_id.trim())
}

#[tauri::command]
pub async fn get_mcp_config() -> Result<McpConfigFile, String> {
    storage::mcp::read_mcp_config()
}

#[tauri::command]
pub async fn save_mcp_config(config: McpConfigFile) -> Result<(), String> {
    storage::mcp::write_mcp_config(&config)
}

#[tauri::command]
pub async fn sync_mcp_catalog() -> Result<(), String> {
    storage::mcp::sync_builtin_catalog()
}

#[tauri::command]
pub async fn set_context7_api_key(api_key: String) -> Result<(), String> {
    let mut config = storage::mcp::read_mcp_config()?;

    // Set the API key in the Context7 server config if it exists
    if let Some(server) = config.servers.get_mut("context7") {
        server.env.insert("CONTEXT7_API_KEY".to_string(), api_key);
        storage::mcp::write_mcp_config(&config)?;
    }

    Ok(())
}
