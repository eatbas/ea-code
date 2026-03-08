use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the OpenCode CLI in non-interactive mode with an explicit model override.
pub async fn run_opencode(
    input: &AgentInput,
    opencode_path: &str,
    model: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        opencode_path,
        &["run", "--model", model, &full_prompt],
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await
}
