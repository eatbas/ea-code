use std::collections::HashMap;

use tauri::{AppHandle, Emitter};

use crate::models::{
    AppSettings, McpCliFixResult, McpCliRuntimeStatus, McpCliServerRuntimeStatus, McpRuntimeStatus,
    McpVerificationConfidence, AI_CLI_NAMES,
};
use crate::storage;

mod install;
mod native;
mod parse;

pub(super) const BUILTIN_SERVER_IDS: [&str; 2] = ["context7", "playwright"];

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCliMcpFixWithPromptRequest {
    pub cli_name: String,
    pub server_id: String,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Fire-and-forget: spawns a parallel check per CLI and returns immediately.
/// Each CLI emits `mcp_cli_runtime_status` as soon as it finishes.
/// When every CLI is done, emits `mcp_runtime_check_complete`.
/// No joining — the frontend never blocks on the slowest CLI.
#[tauri::command]
pub async fn get_mcp_cli_runtime_statuses(app: AppHandle) -> Result<(), String> {
    let settings = storage::settings::read_settings()?;

    let pairs: Vec<(String, String)> = AI_CLI_NAMES
        .iter()
        .filter_map(|cli_name| {
            settings
                .path_for_cli(cli_name)
                .map(|path| (cli_name.to_string(), path.to_string()))
        })
        .collect();

    let total = pairs.len();
    let app_final = app.clone();

    // Single detached task that fans out per-CLI checks.
    tokio::spawn(async move {
        let mut handles = Vec::with_capacity(total);

        for (cli_name, path) in pairs {
            let app_handle = app_final.clone();
            handles.push(tokio::spawn(async move {
                let cli_installed = crate::commands::cli::is_cli_available(&path).await;
                let server_statuses = build_runtime_statuses(&path, cli_installed).await;
                let row = McpCliRuntimeStatus {
                    cli_name,
                    cli_installed,
                    server_statuses,
                };
                let _ = app_handle.emit("mcp_cli_runtime_status", &row);
            }));
        }

        // Wait for all to finish (detached — the command already returned).
        for handle in handles {
            let _ = handle.await;
        }
        let _ = app_final.emit("mcp_runtime_check_complete", ());
    });

    Ok(())
}

/// Fire-and-forget wrapper for tauri command compatibility.
#[tauri::command]
#[allow(dead_code)]
pub async fn get_mcp_cli_runtime_statuses_with_state(app: AppHandle) -> Result<(), String> {
    get_mcp_cli_runtime_statuses(app).await
}

#[tauri::command]
pub async fn run_cli_mcp_fix_with_prompt(
    request: RunCliMcpFixWithPromptRequest,
) -> Result<McpCliFixResult, String> {
    let cli_name = request.cli_name.trim().to_lowercase();
    let server_id = request.server_id.trim().to_lowercase();

    if !AppSettings::is_supported_cli(&cli_name) {
        return Err(format!("Unsupported CLI for MCP fix: {}", request.cli_name));
    }
    if !BUILTIN_SERVER_IDS.contains(&server_id.as_str()) {
        return Err(format!(
            "Unsupported MCP server for fix: {}",
            request.server_id
        ));
    }

    let settings = storage::settings::read_settings()?;
    let cli_path = settings
        .path_for_cli(&cli_name)
        .ok_or_else(|| format!("Unsupported CLI for MCP fix: {}", request.cli_name))?;
    let cli_installed = super::cli::is_cli_available(cli_path).await;
    if !cli_installed {
        return Ok(McpCliFixResult {
            cli_name,
            server_id,
            success: false,
            verification_status: McpRuntimeStatus::NotInstalled,
            verification_confidence: McpVerificationConfidence::None,
            message: Some("CLI is not installed or not reachable in PATH.".to_string()),
            output_summary: "CLI unavailable; no install attempted.".to_string(),
        });
    }

    // Build the server spec from the built-in definition + database env vars.
    let spec = build_server_spec(&server_id)?;

    let (output, used_fallback) = if install::supports_direct_add(&cli_name) {
        // --- Tier 1: deterministic `mcp add` (30 s) ---
        // Works for claude, codex, gemini, kimi.
        let add_result = install::run_mcp_add(cli_path, &cli_name, &spec).await;
        match &add_result {
            Ok(out) if out.status.success() => {
                // Kimi: patch env vars into the config file (mcp add doesn't support -e).
                if cli_name == "kimi" && !spec.env.is_empty() {
                    let _ = install::patch_kimi_env(&spec.server_id, &spec.env);
                }
                (add_result, false)
            }
            _ => (add_result, false),
        }
    } else {
        // --- Tier 2: prompt-based fallback (opencode — interactive-only `mcp add`) ---
        let context7_key = spec.env.get("CONTEXT7_API_KEY").map(String::as_str);
        let model = settings
            .primary_model_for_cli(&cli_name)
            .or_else(|| AppSettings::default_model_for_cli(&cli_name).map(str::to_string))
            .unwrap_or_default();
        let prompt = install::build_fix_prompt(&cli_name, &server_id, context7_key);
        let args = install::build_fix_args(&cli_name, &model, &prompt);
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        (native::run_cli(cli_path, &arg_refs, 120).await, true)
    };

    // --- Verify ---
    let (verification_status, verification_confidence) =
        verify_single_server_runtime(cli_path, &server_id, true).await;

    let command_ok = output.as_ref().is_ok_and(|o| o.status.success());
    let success = command_ok && verification_status == McpRuntimeStatus::Enabled;

    let message = if success {
        None
    } else if !command_ok {
        let hint = if used_fallback {
            "Prompt-based fallback also failed."
        } else {
            "`mcp add` command failed."
        };
        Some(hint.to_string())
    } else {
        Some("Install completed but MCP server is not yet enabled.".to_string())
    };

    let output_summary = match &output {
        Ok(out) => native::summarise_output(out),
        Err(err) => err.clone(),
    };

    Ok(McpCliFixResult {
        cli_name,
        server_id,
        success,
        verification_status,
        verification_confidence,
        message,
        output_summary,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Runs `<cli> mcp list` to determine the status of each built-in server.
/// All 5 CLIs support `mcp list`; the parser handles JSON and plaintext output.
async fn build_runtime_statuses(
    cli_path: &str,
    cli_installed: bool,
) -> Vec<McpCliServerRuntimeStatus> {
    if !cli_installed {
        return BUILTIN_SERVER_IDS
            .iter()
            .map(|server_id| McpCliServerRuntimeStatus {
                server_id: (*server_id).to_string(),
                status: McpRuntimeStatus::NotInstalled,
                verification_confidence: McpVerificationConfidence::None,
                message: None,
            })
            .collect();
    }

    match native::fetch_native_runtime_map(cli_path).await {
        Ok(native_statuses) => BUILTIN_SERVER_IDS
            .iter()
            .map(|server_id| McpCliServerRuntimeStatus {
                server_id: (*server_id).to_string(),
                status: native_statuses
                    .get(*server_id)
                    .cloned()
                    .unwrap_or(McpRuntimeStatus::Disabled),
                verification_confidence: McpVerificationConfidence::Native,
                message: None,
            })
            .collect(),
        Err(err) => BUILTIN_SERVER_IDS
            .iter()
            .map(|server_id| McpCliServerRuntimeStatus {
                server_id: (*server_id).to_string(),
                status: McpRuntimeStatus::Error,
                verification_confidence: McpVerificationConfidence::Native,
                message: Some(err.clone()),
            })
            .collect(),
    }
}

/// Verifies a single server's runtime status after an install attempt.
async fn verify_single_server_runtime(
    cli_path: &str,
    server_id: &str,
    cli_installed: bool,
) -> (McpRuntimeStatus, McpVerificationConfidence) {
    if !cli_installed {
        return (
            McpRuntimeStatus::NotInstalled,
            McpVerificationConfidence::None,
        );
    }
    match native::fetch_native_runtime_map(cli_path).await {
        Ok(map) => (
            map.get(server_id)
                .cloned()
                .unwrap_or(McpRuntimeStatus::Disabled),
            McpVerificationConfidence::Native,
        ),
        Err(_) => (McpRuntimeStatus::Error, McpVerificationConfidence::Native),
    }
}

/// Constructs an `McpServerSpec` from the built-in definitions plus storage env vars.
fn build_server_spec(server_id: &str) -> Result<install::McpServerSpec, String> {
    let (command, args_json) = match server_id {
        "context7" => ("npx", r#"["-y","@upstash/context7-mcp"]"#),
        "playwright" => ("npx", r#"["-y","@playwright/mcp"]"#),
        _ => return Err(format!("Unknown server ID for install: {server_id}")),
    };

    let args: Vec<String> = serde_json::from_str(args_json)
        .map_err(|e| format!("Bad args JSON for {server_id}: {e}"))?;

    let mut env = HashMap::new();
    if server_id == "context7" {
        // Read API key from storage
        if let Ok(mcp_config) = storage::mcp::read_mcp_config() {
            if let Some((_, server)) = mcp_config.servers.iter().find(|(id, _)| *id == "context7") {
                if let Some(key) = server.env.get("CONTEXT7_API_KEY") {
                    let key = key.trim().to_string();
                    if !key.is_empty() {
                        env.insert("CONTEXT7_API_KEY".to_string(), key);
                    }
                }
            }
        }
    }

    Ok(install::McpServerSpec {
        server_id: server_id.to_string(),
        command: command.to_string(),
        args,
        env,
    })
}
