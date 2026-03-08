use serde::{Deserialize, Serialize};

/// Frontend-facing MCP server configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: String,
    pub env: String,
    pub is_enabled: bool,
    pub is_builtin: bool,
    pub cli_bindings: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}
