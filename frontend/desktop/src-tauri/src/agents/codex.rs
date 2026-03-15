use tauri::AppHandle;

use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Codex CLI in non-interactive `exec` mode with full-auto approval.
/// The prompt is piped through stdin (`-` positional arg).
///
/// Flags per <https://developers.openai.com/codex/cli/reference>:
///   exec        Non-interactive execution subcommand
///   --full-auto Shortcut: --ask-for-approval on-request --sandbox workspace-write
///   -m          Model override (global flag)
///   -           Read prompt from stdin
pub async fn run_codex(
    input: &AgentInput,
    codex_path: &str,
    model: &str,
    _session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mut args: Vec<String> = vec!["exec".to_string(), "--full-auto".to_string()];
    if !model.is_empty() {
        args.push("-m".to_string());
        args.push(model.to_string());
    }
    args.push("-".to_string()); // read prompt from stdin
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();

    run_cli_agent(
        codex_path,
        &args_refs,
        None, // no prompt in args; prompt is piped via stdin
        &input.workspace_path,
        app,
        run_id,
        stage,
        Some(&full_prompt),
        &[],
    )
    .await
}
