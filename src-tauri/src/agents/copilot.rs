use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs GitHub Copilot via `gh copilot suggest`.
pub async fn run_copilot(
    input: &AgentInput,
    copilot_path: &str,
    _model: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        copilot_path,
        &["copilot", "suggest", "-t", "code", &full_prompt],
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await
}
