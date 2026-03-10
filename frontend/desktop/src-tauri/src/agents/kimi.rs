use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Kimi CLI in quiet mode with the prompt piped through stdin.
///
/// Flags per <https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html>:
///   --quiet  Shortcut for --print --output-format text --final-message-only
///   -m       Model override
///
/// `PYTHONIOENCODING=utf-8` is set to prevent `[Errno 22] Invalid argument`
/// errors on Windows caused by Python's default encoding handling.
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
    let mut args = vec!["--quiet".to_string()];
    if !model.is_empty() {
        args.push("-m".to_string());
        args.push(model.to_string());
    }
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli_agent(
        kimi_path,
        &args_refs,
        None, // prompt is piped via stdin
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
        Some(&full_prompt),
        &[("PYTHONIOENCODING", "utf-8")],
    )
    .await
}
