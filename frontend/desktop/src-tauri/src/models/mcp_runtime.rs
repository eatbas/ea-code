use serde::{Deserialize, Serialize};

/// Verification confidence for MCP runtime status.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum McpVerificationConfidence {
    Native,
    PromptOnly,
    None,
}

/// Runtime MCP status for a single server in a CLI.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum McpRuntimeStatus {
    Enabled,
    Disabled,
    Unknown,
    NotInstalled,
    Error,
}

/// Runtime MCP status for one server in one CLI.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCliServerRuntimeStatus {
    pub server_id: String,
    pub status: McpRuntimeStatus,
    pub verification_confidence: McpVerificationConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Runtime MCP status for one CLI across built-in servers.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCliRuntimeStatus {
    pub cli_name: String,
    pub cli_installed: bool,
    pub server_statuses: Vec<McpCliServerRuntimeStatus>,
}

/// Result for one prompt-driven MCP fix/install attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCliFixResult {
    pub cli_name: String,
    pub server_id: String,
    pub success: bool,
    pub verification_status: McpRuntimeStatus,
    pub verification_confidence: McpVerificationConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub output_summary: String,
}
