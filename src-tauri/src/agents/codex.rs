use tauri::AppHandle;

use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Codex CLI with the assembled prompt in full-auto mode.
pub async fn run_codex(
    input: &AgentInput,
    codex_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        codex_path,
        &["-q", &full_prompt, "--full-auto"],
        &input.workspace_path,
        app,
        run_id,
        stage,
    )
    .await
}
