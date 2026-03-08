use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};
use super::mcp::build_mcp_config_for_cli;

/// Runs the Codex CLI in full-auto agentic mode with workspace write access.
pub async fn run_codex(
    input: &AgentInput,
    codex_path: &str,
    model: &str,
    session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mcp_config = build_mcp_config_for_cli(db, "codex", session_id);
    let mut args = vec![
        "--full-auto".to_string(),
        "-m".to_string(),
        model.to_string(),
        full_prompt,
    ];
    if let Some(config_path) = mcp_config {
        args.push("--mcp-config".to_string());
        args.push(config_path);
    }
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();

    run_cli_agent(
        codex_path,
        &args_refs,
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
    )
    .await
}
