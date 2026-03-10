use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the OpenCode CLI with the prompt piped through stdin.
///
/// Flags per <https://opencode.ai/docs/cli/>:
///   -m  Model in provider/model format
///
/// The prompt is written to stdin; OpenCode detects piped input and runs
/// non-interactively, returning the result on stdout.
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
    let mut args: Vec<String> = Vec::new();
    if !model.is_empty() {
        args.push("-m".to_string());
        args.push(model.to_string());
    }
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli_agent(
        opencode_path,
        &args_refs,
        None, // prompt is piped via stdin
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
        Some(&full_prompt),
        &[],
    )
    .await
}
