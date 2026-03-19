use std::collections::HashMap;
use std::process::Output;

use super::native;

/// Specification for an MCP server to install via `mcp add`.
pub(super) struct McpServerSpec {
    pub server_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Runs the appropriate `<cli> mcp add` command for a given CLI.
/// Timeout is 30 seconds — enough for npx to download the package.
pub(super) async fn run_mcp_add(
    cli_path: &str,
    cli_name: &str,
    spec: &McpServerSpec,
) -> Result<Output, String> {
    let args = build_add_args(cli_name, spec)?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    native::run_cli(cli_path, &arg_refs, 30).await
}

/// Returns `true` if the CLI supports a deterministic `mcp add` command.
/// OpenCode's `mcp add` is interactive-only, so it requires prompt-based fallback.
pub(super) fn supports_direct_add(cli_name: &str) -> bool {
    matches!(cli_name, "claude" | "codex" | "gemini" | "kimi")
}

/// Dispatches to the correct CLI-specific argument builder.
fn build_add_args(cli_name: &str, spec: &McpServerSpec) -> Result<Vec<String>, String> {
    match cli_name {
        "claude" => Ok(build_claude_args(spec)),
        "gemini" => Ok(build_gemini_args(spec)),
        "kimi" => Ok(build_kimi_args(spec)),
        "codex" => Ok(build_codex_args(spec)),
        _ => Err(format!("No mcp add strategy for CLI: {cli_name}")),
    }
}

/// `claude mcp add-json <id> '<json>' --scope user`
/// Full control via JSON — includes command, args, and env.
fn build_claude_args(spec: &McpServerSpec) -> Vec<String> {
    let mut payload = serde_json::Map::new();
    payload.insert("type".into(), serde_json::Value::String("stdio".into()));
    payload.insert(
        "command".into(),
        serde_json::Value::String(spec.command.clone()),
    );
    payload.insert(
        "args".into(),
        serde_json::Value::Array(
            spec.args
                .iter()
                .map(|a| serde_json::Value::String(a.clone()))
                .collect(),
        ),
    );
    if !spec.env.is_empty() {
        let env_val = serde_json::to_value(&spec.env).unwrap_or_default();
        payload.insert("env".into(), env_val);
    }
    let json_str = serde_json::to_string(&serde_json::Value::Object(payload)).unwrap_or_default();

    vec![
        "mcp".into(),
        "add-json".into(),
        spec.server_id.clone(),
        json_str,
        "--scope".into(),
        "user".into(),
    ]
}

/// `codex mcp add <name> --env KEY=val -- <cmd> <args...>`
/// Env vars passed via repeated `--env` flags before the `--` separator.
fn build_codex_args(spec: &McpServerSpec) -> Vec<String> {
    let mut args = vec!["mcp".into(), "add".into(), spec.server_id.clone()];
    for (key, val) in &spec.env {
        args.push("--env".into());
        args.push(format!("{key}={val}"));
    }
    args.push("--".into());
    args.push(spec.command.clone());
    args.extend(spec.args.clone());
    args
}

/// `gemini mcp add <id> <cmd> [args...] -e KEY=val`
/// Env vars passed via repeated `-e` flags.
fn build_gemini_args(spec: &McpServerSpec) -> Vec<String> {
    let mut args = vec![
        "mcp".into(),
        "add".into(),
        spec.server_id.clone(),
        spec.command.clone(),
    ];
    args.extend(spec.args.clone());
    for (key, val) in &spec.env {
        args.push("-e".into());
        args.push(format!("{key}={val}"));
    }
    args
}

/// `kimi mcp add --transport stdio <id> -- <cmd> [args...]`
/// Env vars are not supported via CLI flags; use `patch_kimi_env()` afterwards.
fn build_kimi_args(spec: &McpServerSpec) -> Vec<String> {
    let mut args = vec![
        "mcp".into(),
        "add".into(),
        "--transport".into(),
        "stdio".into(),
        spec.server_id.clone(),
        "--".into(),
        spec.command.clone(),
    ];
    args.extend(spec.args.clone());
    args
}

/// Patches `~/.kimi/mcp.json` to inject env vars for a server after `kimi mcp add`.
/// Kimi's `mcp add` does not accept `-e` for stdio servers, so we edit the config
/// file directly. Only called when `spec.env` is non-empty.
pub(super) fn patch_kimi_env(server_id: &str, env: &HashMap<String, String>) -> Result<(), String> {
    if env.is_empty() {
        return Ok(());
    }

    let config_path = dirs::home_dir()
        .ok_or("Cannot determine home directory")?
        .join(".kimi")
        .join("mcp.json");

    if !config_path.exists() {
        return Err("Kimi MCP config file not found after install.".into());
    }

    let raw = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Kimi MCP config: {e}"))?;
    let mut config: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse Kimi MCP config: {e}"))?;

    if let Some(servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        if let Some(entry) = servers.get_mut(server_id).and_then(|v| v.as_object_mut()) {
            let env_val = serde_json::to_value(env).unwrap_or_default();
            entry.insert("env".into(), env_val);
        }
    }

    let updated = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialise Kimi MCP config: {e}"))?;
    std::fs::write(&config_path, updated)
        .map_err(|e| format!("Failed to write Kimi MCP config: {e}"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Prompt-based fallback (opencode only — its `mcp add` is interactive)
// ---------------------------------------------------------------------------

/// Builds a natural-language prompt for an AI agent to install/fix an MCP server.
/// Used as fallback for OpenCode (interactive-only `mcp add`) or when direct add fails.
pub(super) fn build_fix_prompt(
    cli_name: &str,
    server_id: &str,
    context7_api_key: Option<&str>,
) -> String {
    let mut lines = vec![
        "Install or fix exactly one MCP server in this CLI.".to_string(),
        format!("Target CLI: {cli_name}"),
        format!("Target MCP server: {server_id}"),
        "Requirements:".to_string(),
        "1) Use global/user-level MCP configuration only.".to_string(),
        "2) Do not use project/workspace-local MCP config files.".to_string(),
        "3) Do not remove, disable, or replace any existing MCP entries.".to_string(),
        "4) Ensure the target MCP server ends enabled.".to_string(),
        "5) Print a short summary of what you changed.".to_string(),
    ];

    if server_id == "context7" {
        let key_line = context7_api_key
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|key| format!("Use CONTEXT7_API_KEY={key} when configuring context7."))
            .unwrap_or_else(|| {
                "CONTEXT7_API_KEY is currently missing; do not invent a key.".to_string()
            });
        lines.push(key_line);
    }

    lines.join("\n")
}

/// Builds CLI arguments for the prompt-based fallback (codex / opencode only).
pub(super) fn build_fix_args(cli_name: &str, model: &str, prompt: &str) -> Vec<String> {
    match cli_name {
        "codex" => vec![
            "--full-auto".to_string(),
            "-m".to_string(),
            model.to_string(),
            prompt.to_string(),
        ],
        "opencode" => vec![
            "run".to_string(),
            "--model".to_string(),
            model.to_string(),
            prompt.to_string(),
        ],
        _ => vec![prompt.to_string()],
    }
}
