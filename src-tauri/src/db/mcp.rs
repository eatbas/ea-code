use std::collections::{BTreeSet, HashMap};

use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::{cli_mcp_bindings, mcp_servers};

use super::models::{
    CliMcpBindingRow, McpServerChangeset, McpServerRow, NewCliMcpBinding, NewMcpServer,
};

pub const MCP_CAPABLE_CLIS: [&str; 2] = ["claude", "codex"];

#[derive(Clone, Debug)]
pub struct ActiveMcpServer {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

pub fn list_servers(pool: &DbPool) -> Result<Vec<McpServerRow>, String> {
    let mut conn = super::get_conn(pool)?;
    mcp_servers::table
        .order((mcp_servers::is_builtin.desc(), mcp_servers::name.asc()))
        .load::<McpServerRow>(&mut conn)
        .map_err(|e| format!("Failed to list MCP servers: {e}"))
}

pub fn list_bindings(pool: &DbPool) -> Result<Vec<CliMcpBindingRow>, String> {
    let mut conn = super::get_conn(pool)?;
    cli_mcp_bindings::table
        .order((cli_mcp_bindings::cli_name.asc(), cli_mcp_bindings::mcp_server_id.asc()))
        .load::<CliMcpBindingRow>(&mut conn)
        .map_err(|e| format!("Failed to list MCP bindings: {e}"))
}

pub fn create_custom_server(pool: &DbPool, new_server: &NewMcpServer<'_>) -> Result<McpServerRow, String> {
    let mut conn = super::get_conn(pool)?;
    diesel::insert_into(mcp_servers::table)
        .values(new_server)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to create MCP server: {e}"))?;
    mcp_servers::table
        .find(new_server.id)
        .first::<McpServerRow>(&mut conn)
        .map_err(|e| format!("Failed to load created MCP server: {e}"))
}

pub fn update_custom_server(
    pool: &DbPool,
    server_id: &str,
    changeset: &McpServerChangeset<'_>,
) -> Result<McpServerRow, String> {
    let mut conn = super::get_conn(pool)?;
    let row = mcp_servers::table
        .find(server_id)
        .first::<McpServerRow>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to load MCP server: {e}"))?
        .ok_or_else(|| "MCP server not found".to_string())?;
    if row.is_builtin {
        return Err("Built-in MCP servers cannot be edited".to_string());
    }

    diesel::update(mcp_servers::table.find(server_id))
        .set(changeset)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update MCP server: {e}"))?;

    mcp_servers::table
        .find(server_id)
        .first::<McpServerRow>(&mut conn)
        .map_err(|e| format!("Failed to load updated MCP server: {e}"))
}

pub fn delete_custom_server(pool: &DbPool, server_id: &str) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let row = mcp_servers::table
        .find(server_id)
        .first::<McpServerRow>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to load MCP server: {e}"))?
        .ok_or_else(|| "MCP server not found".to_string())?;
    if row.is_builtin {
        return Err("Built-in MCP servers cannot be deleted".to_string());
    }

    diesel::delete(mcp_servers::table.find(server_id))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete MCP server: {e}"))?;
    Ok(())
}

pub fn set_server_enabled(pool: &DbPool, server_id: &str, enabled: bool) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;
    let now = chrono::Utc::now().to_rfc3339();
    let affected = diesel::update(mcp_servers::table.find(server_id))
        .set((mcp_servers::is_enabled.eq(enabled), mcp_servers::updated_at.eq(now)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to toggle MCP server: {e}"))?;
    if affected == 0 {
        return Err("MCP server not found".to_string());
    }
    Ok(())
}

pub fn replace_bindings(pool: &DbPool, server_id: &str, cli_names: &[String]) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    let filtered = cli_names
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|cli| MCP_CAPABLE_CLIS.contains(&cli.as_str()))
        .collect::<BTreeSet<_>>();

    conn.transaction(|conn| {
        diesel::delete(cli_mcp_bindings::table.filter(cli_mcp_bindings::mcp_server_id.eq(server_id)))
            .execute(conn)?;

        if filtered.is_empty() {
            return Ok::<(), diesel::result::Error>(());
        }

        let new_rows = filtered
            .iter()
            .map(|cli| NewCliMcpBinding {
                cli_name: cli.as_str(),
                mcp_server_id: server_id,
            })
            .collect::<Vec<_>>();

        diesel::insert_into(cli_mcp_bindings::table)
            .values(&new_rows)
            .execute(conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| format!("Failed to update MCP bindings: {e}"))?;

    Ok(())
}

pub fn get_active_servers_for_cli(pool: &DbPool, cli_name: &str) -> Result<Vec<ActiveMcpServer>, String> {
    let mut conn = super::get_conn(pool)?;
    let cli = cli_name.trim().to_lowercase();

    let rows = mcp_servers::table
        .inner_join(cli_mcp_bindings::table.on(cli_mcp_bindings::mcp_server_id.eq(mcp_servers::id)))
        .filter(mcp_servers::is_enabled.eq(true))
        .filter(cli_mcp_bindings::cli_name.eq(cli))
        .select(McpServerRow::as_select())
        .load::<McpServerRow>(&mut conn)
        .map_err(|e| format!("Failed to load active MCP servers: {e}"))?;

    rows.into_iter().map(parse_active_server).collect()
}

fn parse_active_server(row: McpServerRow) -> Result<ActiveMcpServer, String> {
    let args = serde_json::from_str::<Vec<String>>(&row.args)
        .map_err(|e| format!("Invalid args JSON for MCP server {}: {e}", row.id))?;
    let env = serde_json::from_str::<HashMap<String, String>>(&row.env)
        .map_err(|e| format!("Invalid env JSON for MCP server {}: {e}", row.id))?;
    Ok(ActiveMcpServer {
        id: row.id,
        command: row.command,
        args,
        env,
    })
}
