use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Kimi CLI in headless print mode for non-interactive agentic execution.
pub async fn run_kimi(
    input: &AgentInput,
    kimi_path: &str,
    model: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        kimi_path,
        &["--print", "-p", &full_prompt, "--model", model],
        Some(2), // prompt is at index 2: ["--print", "-p", prompt, ...]
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await
}
