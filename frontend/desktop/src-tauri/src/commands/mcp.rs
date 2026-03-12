use std::collections::HashMap;

use tauri::State;

use crate::db;
use crate::models::McpServer;

use super::AppState;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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

fn to_model(row: db::models::McpServerRow, bindings: Vec<String>) -> McpServer {
    McpServer {
        id: row.id,
        name: row.name,
        description: row.description,
        command: row.command,
        args: row.args,
        env: row.env,
        is_enabled: row.is_enabled,
        is_builtin: row.is_builtin,
        cli_bindings: bindings,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
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

#[tauri::command]
pub async fn list_mcp_servers(state: State<'_, AppState>) -> Result<Vec<McpServer>, String> {
    let rows = db::mcp::list_servers(&state.db)?;
    let bindings = db::mcp::list_bindings(&state.db)?;

    let mut by_server = HashMap::<String, Vec<String>>::new();
    for binding in bindings {
        by_server
            .entry(binding.mcp_server_id)
            .or_default()
            .push(binding.cli_name);
    }

    Ok(rows
        .into_iter()
        .map(|row| {
            let b = by_server.remove(&row.id).unwrap_or_default();
            to_model(row, b)
        })
        .collect())
}

#[tauri::command]
pub async fn list_mcp_capable_clis(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let settings = db::settings::get(&state.db)?;
    let mut available = Vec::new();

    for cli_name in db::mcp::MCP_CAPABLE_CLIS {
        let Some(path) = settings.path_for_cli(cli_name) else {
            continue;
        };

        if super::cli::is_cli_available(path).await {
            available.push(cli_name.to_string());
        }
    }

    Ok(available)
}

#[tauri::command]
pub async fn set_mcp_server_enabled(
    state: State<'_, AppState>,
    server_id: String,
    enabled: bool,
) -> Result<(), String> {
    db::mcp::set_server_enabled(&state.db, server_id.trim(), enabled)
}

#[tauri::command]
pub async fn set_mcp_server_bindings(
    state: State<'_, AppState>,
    server_id: String,
    cli_names: Vec<String>,
) -> Result<(), String> {
    db::mcp::replace_bindings(&state.db, server_id.trim(), &cli_names)
}

#[tauri::command]
pub async fn create_mcp_server(
    state: State<'_, AppState>,
    payload: CreateMcpServerPayload,
) -> Result<McpServer, String> {
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
    let args = normalise_json_array(payload.args.as_deref())?;
    let env = normalise_json_object(payload.env.as_deref())?;
    let is_enabled = payload.is_enabled.unwrap_or(false);

    let row = db::mcp::create_custom_server(
        &state.db,
        &db::models::NewMcpServer {
            id: &id,
            name: &name,
            description: &description,
            command: &command,
            args: &args,
            env: &env,
            is_enabled,
            is_builtin: false,
        },
    )?;

    let bindings = payload.cli_bindings.unwrap_or_default();
    db::mcp::replace_bindings(&state.db, &id, &bindings)?;
    Ok(to_model(row, bindings))
}

#[tauri::command]
pub async fn update_mcp_server(
    state: State<'_, AppState>,
    payload: UpdateMcpServerPayload,
) -> Result<McpServer, String> {
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
    let args = normalise_json_array(payload.args.as_deref())?;
    let env = normalise_json_object(payload.env.as_deref())?;
    let is_enabled = payload.is_enabled.unwrap_or(false);
    let now = crate::db::now_rfc3339();

    let row = db::mcp::update_custom_server(
        &state.db,
        &id,
        &db::models::McpServerChangeset {
            name: &name,
            description: &description,
            command: &command,
            args: &args,
            env: &env,
            is_enabled,
            updated_at: &now,
        },
    )?;

    let bindings = payload.cli_bindings.unwrap_or_default();
    db::mcp::replace_bindings(&state.db, &id, &bindings)?;
    Ok(to_model(row, bindings))
}

#[tauri::command]
pub async fn delete_mcp_server(state: State<'_, AppState>, server_id: String) -> Result<(), String> {
    db::mcp::delete_custom_server(&state.db, server_id.trim())
}

#[tauri::command]
pub async fn set_context7_api_key(
    state: State<'_, AppState>,
    api_key: String,
) -> Result<(), String> {
    db::mcp::set_server_env_var(
        &state.db,
        "context7",
        "CONTEXT7_API_KEY",
        Some(api_key.as_str()),
    )
}
