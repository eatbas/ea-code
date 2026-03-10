use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Gemini CLI with the prompt piped through stdin.
///
/// Flags per <https://github.com/google-gemini/gemini-cli>:
///   --model  Specify which Gemini model to use
///   --yolo   Auto-approve all tool calls
///
/// The prompt is written to stdin; Gemini reads it when stdin is piped
/// (non-TTY), producing output then exiting.
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
    let mut args: Vec<String> = Vec::new();
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    args.push("--yolo".to_string());
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli_agent(
        gemini_path,
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
