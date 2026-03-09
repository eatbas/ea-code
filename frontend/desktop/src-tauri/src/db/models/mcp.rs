use diesel::prelude::*;
use serde::Serialize;

use crate::schema::{cli_mcp_bindings, mcp_servers};

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = mcp_servers)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: String,
    pub env: String,
    pub is_enabled: bool,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = mcp_servers)]
pub struct NewMcpServer<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub command: &'a str,
    pub args: &'a str,
    pub env: &'a str,
    pub is_enabled: bool,
    pub is_builtin: bool,
}

#[derive(AsChangeset)]
#[diesel(table_name = mcp_servers)]
pub struct McpServerChangeset<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub command: &'a str,
    pub args: &'a str,
    pub env: &'a str,
    pub is_enabled: bool,
    pub updated_at: &'a str,
}

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = cli_mcp_bindings)]
#[serde(rename_all = "camelCase")]
pub struct CliMcpBindingRow {
    pub id: i32,
    pub cli_name: String,
    pub mcp_server_id: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = cli_mcp_bindings)]
pub struct NewCliMcpBinding<'a> {
    pub cli_name: &'a str,
    pub mcp_server_id: &'a str,
}
