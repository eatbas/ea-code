use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};
use super::mcp::build_mcp_config_for_cli;

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

/// Runs the Claude CLI in full agentic mode with tool access.
/// When the MCP binary is available, passes `--mcp-config` so the agent
/// can query session history during execution.
pub async fn run_claude(
    input: &AgentInput,
    claude_path: &str,
    model: &str,
    agent_max_turns: u32,
    session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mcp_config = build_mcp_config_for_cli(db, "claude", session_id);

    let mut args: Vec<String> = vec![
        "-p".to_string(),
        full_prompt,
        "--model".to_string(),
        model.to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--allowedTools".to_string(),
        "Bash,Edit,Read,Write,Glob,Grep".to_string(),
        "--max-turns".to_string(),
        agent_max_turns.to_string(),
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
