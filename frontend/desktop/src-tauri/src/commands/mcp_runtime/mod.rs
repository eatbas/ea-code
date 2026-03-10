use tauri::State;

use crate::db;
use crate::models::{
    AppSettings, McpCliFixResult, McpCliRuntimeStatus, McpCliServerRuntimeStatus, McpRuntimeStatus,
    McpVerificationConfidence,
};

use super::AppState;

mod native;
mod parse;

const AI_MCP_CLIS: [&str; 5] = ["claude", "codex", "gemini", "kimi", "opencode"];
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
    let mut rows = Vec::with_capacity(AI_MCP_CLIS.len());

    for cli_name in AI_MCP_CLIS {
        let path = cli_path_for_name(&settings, cli_name);
        let cli_installed = super::cli::is_cli_available(path).await;
        let server_statuses = build_runtime_statuses(path, cli_name, cli_installed).await;
        rows.push(McpCliRuntimeStatus {
            cli_name: cli_name.to_string(),
            cli_installed,
            server_statuses,
        });
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

    if !AI_MCP_CLIS.contains(&cli_name.as_str()) {
        return Err(format!("Unsupported CLI for MCP fix: {}", request.cli_name));
    }
    if !BUILTIN_SERVER_IDS.contains(&server_id.as_str()) {
        return Err(format!("Unsupported MCP server for fix: {}", request.server_id));
    }

    let settings = db::settings::get(&state.db)?;
    let cli_path = cli_path_for_name(&settings, &cli_name);
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

    let model = default_model_for_cli(&settings, &cli_name);
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

fn cli_path_for_name<'a>(settings: &'a AppSettings, cli_name: &str) -> &'a str {
    match cli_name {
        "claude" => settings.claude_path.as_str(),
        "codex" => settings.codex_path.as_str(),
        "gemini" => settings.gemini_path.as_str(),
        "kimi" => settings.kimi_path.as_str(),
        "opencode" => settings.opencode_path.as_str(),
        _ => "",
    }
}

fn default_model_for_cli(settings: &AppSettings, cli_name: &str) -> String {
    let csv = match cli_name {
        "claude" => settings.claude_model.as_str(),
        "codex" => settings.codex_model.as_str(),
        "gemini" => settings.gemini_model.as_str(),
        "kimi" => settings.kimi_model.as_str(),
        "opencode" => settings.opencode_model.as_str(),
        _ => "",
    };
    let first = csv.split(',').next().unwrap_or("").trim();
    if !first.is_empty() {
        return first.to_string();
    }
    match cli_name {
        "claude" => "sonnet".to_string(),
        "codex" => "codex-5.3".to_string(),
        "gemini" => "gemini-2.5-pro".to_string(),
        "kimi" => "kimi-code".to_string(),
        "opencode" => "opencode/glm-5".to_string(),
        _ => String::new(),
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
