pub(crate) mod api_health;
pub(crate) mod cli;
pub(crate) mod cli_http;
pub(crate) mod cli_util;
pub(crate) mod cli_version;
#[cfg(target_os = "windows")]
pub(crate) mod git_bash;
pub(crate) mod mcp;
pub(crate) mod mcp_runtime;
pub(crate) mod settings;
pub(crate) mod workspace;

/// Shared application state.
pub struct AppState {}
