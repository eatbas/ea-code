use serde::{Deserialize, Serialize};

/// Workspace validation result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub path: String,
    pub is_git_repo: bool,
    pub is_dirty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// CLI health check result per binary.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub available: bool,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregate CLI health check result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliHealth {
    pub claude: CliStatus,
    pub codex: CliStatus,
    pub gemini: CliStatus,
    pub kimi: CliStatus,
    pub opencode: CliStatus,
}

/// Version and availability information for a single CLI tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliVersionInfo {
    pub name: String,
    pub cli_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub up_to_date: bool,
    pub update_command: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregate version information for all CLI tools.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllCliVersions {
    pub claude: CliVersionInfo,
    pub codex: CliVersionInfo,
    pub gemini: CliVersionInfo,
    pub kimi: CliVersionInfo,
    pub opencode: CliVersionInfo,
}
