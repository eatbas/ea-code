use std::collections::HashMap;

use crate::models::AI_CLI_NAMES;
use crate::models::{McpConfigFile, McpServerConfig};

use super::{atomic_write, config_dir, with_mcp_lock};

const MCP_FILE: &str = "mcp.json";
const SCHEMA_VERSION: u32 = 1;

/// Built-in MCP server specifications.
#[allow(dead_code)]
struct BuiltinMcpSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    command: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
    bindings: &'static [&'static str],
}

const BUILTIN_MCP_SERVERS: &[BuiltinMcpSpec] = &[
    BuiltinMcpSpec {
        id: "context7",
        name: "Context7",
        description: "Library and API documentation lookup.",
        command: "npx",
        args: &["-y", "@upstash/context7-mcp"],
        env: &[],
        bindings: &["claude", "codex"],
    },
    BuiltinMcpSpec {
        id: "playwright",
        name: "Playwright",
        description: "Browser automation and web testing tools.",
        command: "npx",
        args: &["-y", "@playwright/mcp"],
        env: &[],
        bindings: &["claude", "codex"],
    },
];

/// Reads MCP configuration from mcp.json.
/// Returns default config if the file doesn't exist.
pub fn read_mcp_config() -> Result<McpConfigFile, String> {
    let path = config_dir()?.join(MCP_FILE);

    if !path.exists() {
        return Ok(default_mcp_config());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read MCP config file: {e}"))?;

    let config: McpConfigFile = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse MCP config file: {e}"))?;

    Ok(config)
}

/// Writes MCP configuration to mcp.json atomically.
/// H8: Protected by file lock for concurrent access.
pub fn write_mcp_config(config: &McpConfigFile) -> Result<(), String> {
    with_mcp_lock(|| {
        let path = config_dir()?.join(MCP_FILE);

        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialise MCP config: {e}"))?;

        atomic_write(&path, &json)
    })
}

/// Syncs built-in MCP servers into the config file.
/// Adds any missing built-in servers, but preserves user modifications.
/// Also repairs partially removed bindings (where server exists but binding doesn't).
/// H8: Protected by file lock for concurrent access.
pub fn sync_builtin_catalog() -> Result<(), String> {
    with_mcp_lock(|| {
        let mut config = read_mcp_config()?;

        for spec in BUILTIN_MCP_SERVERS {
            let server_exists = config.servers.contains_key(spec.id);

            if !server_exists {
                // Add missing server
                let server_config = McpServerConfig {
                    command: spec.command.to_string(),
                    args: spec.args.iter().map(|s| s.to_string()).collect(),
                    env: spec
                        .env
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                };
                config.servers.insert(spec.id.to_string(), server_config);
            }

            // Repair bindings: ensure server is bound to all its default CLIs
            for cli in spec.bindings {
                let bindings = config.cli_bindings.entry(cli.to_string()).or_default();
                if !bindings.contains(&spec.id.to_string()) {
                    bindings.push(spec.id.to_string());
                }
            }
        }

        // Write directly inside lock to avoid deadlock
        let path = config_dir()?.join(MCP_FILE);
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialise MCP config: {e}"))?;
        atomic_write(&path, &json)
    })
}

fn default_mcp_config() -> McpConfigFile {
    McpConfigFile {
        schema_version: SCHEMA_VERSION,
        servers: HashMap::new(),
        cli_bindings: HashMap::new(),
    }
}

/// Gets active MCP servers for a specific CLI.
pub fn get_active_servers_for_cli(
    cli_name: &str,
) -> Result<Vec<(String, McpServerConfig)>, String> {
    let config = read_mcp_config()?;
    let cli = cli_name.trim().to_lowercase();

    let server_ids = config.cli_bindings.get(&cli).cloned().unwrap_or_default();

    let mut servers = Vec::new();
    for id in server_ids {
        if let Some(server_config) = config.servers.get(&id) {
            servers.push((id.clone(), server_config.clone()));
        }
    }

    Ok(servers)
}

/// Adds or updates a custom MCP server.
/// H8: Protected by file lock for concurrent access.
pub fn add_server(id: &str, config: &McpServerConfig) -> Result<(), String> {
    with_mcp_lock(|| {
        let mut mcp_config = read_mcp_config()?;
        mcp_config.servers.insert(id.to_string(), config.clone());

        let path = config_dir()?.join(MCP_FILE);
        let json = serde_json::to_string_pretty(&mcp_config)
            .map_err(|e| format!("Failed to serialise MCP config: {e}"))?;
        atomic_write(&path, &json)
    })
}

/// Removes an MCP server.
/// H8: Protected by file lock for concurrent access.
pub fn remove_server(id: &str) -> Result<(), String> {
    with_mcp_lock(|| {
        let mut config = read_mcp_config()?;

        if !config.servers.contains_key(id) {
            return Err(format!("MCP server not found: {id}"));
        }

        config.servers.remove(id);

        // Remove from all CLI bindings
        for bindings in config.cli_bindings.values_mut() {
            bindings.retain(|b| b != id);
        }

        // Remove empty binding entries
        config
            .cli_bindings
            .retain(|_, bindings| !bindings.is_empty());

        let path = config_dir()?.join(MCP_FILE);
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialise MCP config: {e}"))?;
        atomic_write(&path, &json)
    })
}

/// Validates a CLI name.
fn validate_cli_name(cli: &str) -> Result<(), String> {
    let cli = cli.trim().to_lowercase();
    if cli.is_empty() {
        return Err("CLI name cannot be empty".to_string());
    }
    if !cli
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "Invalid CLI name '{cli}': only alphanumeric, hyphens, and underscores allowed"
        ));
    }
    Ok(())
}

/// Sets CLI bindings for a server (replaces existing).
/// H8: Protected by file lock for concurrent access.
pub fn set_server_bindings(server_id: &str, cli_names: &[String]) -> Result<(), String> {
    with_mcp_lock(|| {
        let mut config = read_mcp_config()?;

        if !config.servers.contains_key(server_id) {
            return Err(format!("MCP server not found: {server_id}"));
        }

        // Validate all CLI names first
        for cli in cli_names {
            validate_cli_name(cli)?;
        }

        // Remove server from all existing bindings
        for bindings in config.cli_bindings.values_mut() {
            bindings.retain(|b| b != server_id);
        }

        // Add to new bindings
        let filtered: Vec<String> = cli_names
            .iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|cli| AI_CLI_NAMES.contains(&cli.as_str()))
            .collect();

        for cli in filtered {
            config
                .cli_bindings
                .entry(cli)
                .or_default()
                .push(server_id.to_string());
        }

        // Remove empty binding entries
        config
            .cli_bindings
            .retain(|_, bindings| !bindings.is_empty());

        let path = config_dir()?.join(MCP_FILE);
        let json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialise MCP config: {e}"))?;
        atomic_write(&path, &json)
    })
}

/// Gets an environment variable for a specific MCP server.
/// Returns None if the server or variable doesn't exist.
pub fn get_server_env_var(server_id: &str, var_name: &str) -> Result<Option<String>, String> {
    let config = read_mcp_config()?;

    if let Some(server_config) = config.servers.get(server_id) {
        Ok(server_config.env.get(var_name).cloned())
    } else {
        Ok(None)
    }
}
