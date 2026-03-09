use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Gemini CLI in agentic mode with auto-approved tool use.
pub async fn run_gemini(
    input: &AgentInput,
    gemini_path: &str,
    model: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);
    run_cli_agent(
        gemini_path,
        &["-p", &full_prompt, "-m", model, "--yolo"],
        Some(1), // prompt is at index 1: ["-p", prompt, ...]
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await
}
