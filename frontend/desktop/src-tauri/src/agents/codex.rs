use tauri::AppHandle;

use crate::models::{PipelineStage, StageExecutionIntent};

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs the Codex CLI in non-interactive `exec` mode.
/// The prompt is piped through stdin (`-` positional arg).
///
/// Flags per <https://developers.openai.com/codex/cli/reference>:
///   exec        Non-interactive execution subcommand
///   --full-auto Convenience alias for automatic workspace-write execution
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
    intent: StageExecutionIntent,
    output_file: Option<&str>,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mut args: Vec<String> = vec!["exec".to_string()];
    if !model.is_empty() {
        args.push("-m".to_string());
        args.push(model.to_string());
    }
    match intent {
        StageExecutionIntent::Code => {
            args.push("--full-auto".to_string());
        }
        StageExecutionIntent::Text => {
            args.extend([
                "--sandbox".to_string(),
                "read-only".to_string(),
            ]);
            if let Some(path) = output_file {
                args.push("--output-last-message".to_string());
                args.push(path.to_string());
            }
        }
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
