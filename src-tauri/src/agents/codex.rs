use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Codex CLI in full-auto agentic mode with workspace write access.
pub async fn run_codex(
    input: &AgentInput,
    codex_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        codex_path,
        &["--full-auto", &full_prompt],
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await
}
