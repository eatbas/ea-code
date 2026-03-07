use tauri::AppHandle;

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

/// Runs the Claude CLI in full agentic mode with tool access.
pub async fn run_claude(
    input: &AgentInput,
    claude_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    let mut output = run_cli_agent(
        claude_path,
        &[
            "-p",
            &full_prompt,
            "--output-format",
            "json",
            "--allowedTools",
            "Bash,Edit,Read,Write,Glob,Grep",
        ],
        &input.workspace_path,
        app,
        run_id,
        stage,
    )
    .await?;

    output.raw_text = extract_claude_text(&output.raw_text);
    Ok(output)
}
