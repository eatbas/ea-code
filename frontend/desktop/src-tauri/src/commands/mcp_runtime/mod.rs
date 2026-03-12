use tauri::State;

use crate::db;
use crate::models::{
    AppSettings, McpCliFixResult, McpCliRuntimeStatus, McpCliServerRuntimeStatus,
    McpRuntimeStatus, McpVerificationConfidence, AI_CLI_NAMES,
};

use super::AppState;

mod native;
mod parse;

pub(super) const BUILTIN_SERVER_IDS: [&str; 2] = ["context7", "playwright"];

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCliMcpFixWithPromptRequest {
    pub cli_name: String,
    pub server_id: String,
}

#[tauri::command]
pub async fn get_mcp_cli_runtime_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<McpCliRuntimeStatus>, String> {
    let settings = db::settings::get(&state.db)?;

    // Collect CLI name/path pairs, then check all in parallel.
    let pairs: Vec<(&str, &str)> = AI_CLI_NAMES
        .iter()
        .filter_map(|cli_name| {
            settings.path_for_cli(cli_name).map(|path| (*cli_name, path))
        })
        .collect();

    let mut handles = Vec::with_capacity(pairs.len());
    for (cli_name, path) in pairs {
        let path = path.to_string();
        let cli_name = cli_name.to_string();
        handles.push(tokio::spawn(async move {
            let cli_installed = crate::commands::cli::is_cli_available(&path).await;
            let server_statuses = build_runtime_statuses(&path, &cli_name, cli_installed).await;
            McpCliRuntimeStatus {
                cli_name,
                cli_installed,
                server_statuses,
            }
        }));
    }

    let mut rows = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(row) => rows.push(row),
            Err(e) => return Err(format!("Runtime status task failed: {e}")),
        }
    }
    Ok(rows)
}

#[tauri::command]
pub async fn run_cli_mcp_fix_with_prompt(
    state: State<'_, AppState>,
    request: RunCliMcpFixWithPromptRequest,
) -> Result<McpCliFixResult, String> {
    let cli_name = request.cli_name.trim().to_lowercase();
    let server_id = request.server_id.trim().to_lowercase();

    if !AppSettings::is_supported_cli(&cli_name) {
        return Err(format!("Unsupported CLI for MCP fix: {}", request.cli_name));
    }
    if !BUILTIN_SERVER_IDS.contains(&server_id.as_str()) {
        return Err(format!("Unsupported MCP server for fix: {}", request.server_id));
    }

    let settings = db::settings::get(&state.db)?;
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
            output_summary: "CLI unavailable; no prompt run attempted.".to_string(),
        });
    }

    let context7_api_key = if server_id == "context7" {
        db::mcp::get_server_env_var(&state.db, "context7", "CONTEXT7_API_KEY")?
    } else {
        None
    };

    let model = settings
        .primary_model_for_cli(&cli_name)
        .or_else(|| AppSettings::default_model_for_cli(&cli_name).map(str::to_string))
        .unwrap_or_default();
    let prompt = build_fix_prompt(&cli_name, &server_id, context7_api_key.as_deref());
    let args = build_fix_args(&cli_name, &model, &prompt);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = native::run_cli(cli_path, &arg_refs, 240).await?;

    let (verification_status, verification_confidence) =
        verify_single_server_runtime(cli_path, &cli_name, &server_id, true).await;
    let command_ok = output.status.success();
    let success = if matches!(verification_confidence, McpVerificationConfidence::Native) {
        command_ok && verification_status == McpRuntimeStatus::Enabled
    } else {
        command_ok
    };

    let message = if success {
        None
    } else if !command_ok {
        Some("CLI prompt execution failed.".to_string())
    } else if verification_status != McpRuntimeStatus::Enabled {
        Some("CLI completed, but native MCP verification is not enabled yet.".to_string())
    } else {
        Some("MCP fix did not complete successfully.".to_string())
    };

    Ok(McpCliFixResult {
        cli_name,
        server_id,
        success,
        verification_status,
        verification_confidence,
        message,
        output_summary: native::summarise_output(&output),
    })
}

async fn build_runtime_statuses(
    cli_path: &str,
    cli_name: &str,
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

    if matches!(cli_name, "claude" | "codex") {
        return match native::fetch_native_runtime_map(cli_path).await {
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
        };
    }

    BUILTIN_SERVER_IDS
        .iter()
        .map(|server_id| McpCliServerRuntimeStatus {
            server_id: (*server_id).to_string(),
            status: McpRuntimeStatus::Unknown,
            verification_confidence: McpVerificationConfidence::PromptOnly,
            message: Some("Runtime MCP introspection is unavailable for this CLI.".to_string()),
        })
        .collect()
}

async fn verify_single_server_runtime(
    cli_path: &str,
    cli_name: &str,
    server_id: &str,
    cli_installed: bool,
) -> (McpRuntimeStatus, McpVerificationConfidence) {
    if !cli_installed {
        return (McpRuntimeStatus::NotInstalled, McpVerificationConfidence::None);
    }
    if !matches!(cli_name, "claude" | "codex") {
        return (McpRuntimeStatus::Unknown, McpVerificationConfidence::PromptOnly);
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

fn build_fix_prompt(cli_name: &str, server_id: &str, context7_api_key: Option<&str>) -> String {
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
                "CONTEXT7_API_KEY is currently missing; do not invent a key and keep existing values if present.".to_string()
            });
        lines.push(key_line);
    }

    lines.join("\n")
}

fn build_fix_args(cli_name: &str, model: &str, prompt: &str) -> Vec<String> {
    match cli_name {
        "claude" => vec![
            "-p".to_string(),
            prompt.to_string(),
            "--model".to_string(),
            model.to_string(),
            "--output-format".to_string(),
            "json".to_string(),
            "--max-turns".to_string(),
            "25".to_string(),
            "--allowedTools".to_string(),
            "Bash,Read,Write,Edit,Glob,Grep".to_string(),
        ],
        "codex" => vec![
            "--full-auto".to_string(),
            "-m".to_string(),
            model.to_string(),
            prompt.to_string(),
        ],
        "gemini" => vec![
            "-p".to_string(),
            prompt.to_string(),
            "-m".to_string(),
            model.to_string(),
            "--yolo".to_string(),
        ],
        "kimi" => vec![
            "--print".to_string(),
            "-p".to_string(),
            prompt.to_string(),
            "--model".to_string(),
            model.to_string(),
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
