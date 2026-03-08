use std::io::Write;

use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Attempts to extract the textual result from Claude's JSON output.
/// Falls back to the raw string if parsing fails.
fn extract_claude_text(raw: &str) -> String {
    #[derive(serde::Deserialize)]
    struct ClaudeJsonOutput {
        result: Option<String>,
    }

    serde_json::from_str::<ClaudeJsonOutput>(raw)
        .ok()
        .and_then(|parsed| parsed.result)
        .unwrap_or_else(|| raw.to_string())
}

/// Locates the `ea-code-mcp` binary next to the current executable,
/// or falls back to searching PATH.
fn mcp_binary_path() -> Option<String> {
    // Try adjacent to current executable first
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("ea-code-mcp");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    // Fall back to PATH lookup
    if let Ok(output) = std::process::Command::new("which")
        .arg("ea-code-mcp")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

/// Writes a temporary MCP config JSON file for the Claude CLI and returns
/// its path. Returns `None` if the MCP binary cannot be found.
fn write_mcp_config(session_id: Option<&str>) -> Option<String> {
    let mcp_bin = mcp_binary_path()?;

    let mut args = Vec::new();
    if let Some(sid) = session_id {
        args.push("--session-id".to_string());
        args.push(sid.to_string());
    }

    let config = serde_json::json!({
        "mcpServers": {
            "ea-code": {
                "command": mcp_bin,
                "args": args,
            }
        }
    });

    let config_dir = dirs::config_dir()?.join("ea-code");
    let config_path = config_dir.join("mcp-config.json");

    let mut file = std::fs::File::create(&config_path).ok()?;
    file.write_all(config.to_string().as_bytes()).ok()?;

    Some(config_path.to_string_lossy().to_string())
}

/// Runs the Claude CLI in full agentic mode with tool access.
/// When the MCP binary is available, passes `--mcp-config` so the agent
/// can query session history during execution.
pub async fn run_claude(
    input: &AgentInput,
    claude_path: &str,
    model: &str,
    session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mcp_config = write_mcp_config(session_id);

    let mut args: Vec<String> = vec![
        "-p".to_string(),
        full_prompt,
        "--model".to_string(),
        model.to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--allowedTools".to_string(),
        "Bash,Edit,Read,Write,Glob,Grep".to_string(),
    ];

    if let Some(ref config_path) = mcp_config {
        args.push("--mcp-config".to_string());
        args.push(config_path.clone());
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let mut output = run_cli_agent(
        claude_path,
        &args_refs,
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await?;

    output.raw_text = extract_claude_text(&output.raw_text);
    Ok(output)
}
