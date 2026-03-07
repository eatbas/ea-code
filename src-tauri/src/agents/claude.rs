use tauri::AppHandle;

use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Claude CLI with the assembled prompt.
pub async fn run_claude(
    input: &AgentInput,
    claude_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        claude_path,
        &["-p", &full_prompt, "--output-format", "text"],
        &input.workspace_path,
        app,
        run_id,
        stage,
    )
    .await
}
