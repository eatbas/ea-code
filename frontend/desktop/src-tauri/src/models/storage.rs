use serde::{Deserialize, Serialize};

/// Project entry in the projects.json array.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub is_git_repo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// MCP server configuration entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub command: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub env: std::collections::HashMap<String, String>,
}

/// MCP configuration file (mcp.json) - servers and CLI bindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfigFile {
    pub schema_version: u32,
    #[serde(default)]
    pub servers: std::collections::HashMap<String, McpServerConfig>,
    /// CLI bindings map: CLI name -> list of MCP server IDs.
    #[serde(default)]
    pub cli_bindings: std::collections::HashMap<String, Vec<String>>,
}
