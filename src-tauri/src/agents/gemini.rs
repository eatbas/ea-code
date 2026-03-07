use tauri::AppHandle;

use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Gemini CLI with the assembled prompt.
pub async fn run_gemini(
    input: &AgentInput,
    gemini_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        gemini_path,
        &["-p", &full_prompt],
        &input.workspace_path,
        app,
        run_id,
        stage,
    )
    .await
}
