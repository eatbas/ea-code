use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs Claude Code in non-interactive print mode.
///
/// Flags per <https://code.claude.com/docs/en/cli-reference>:
///   --print                        Non-interactive execution
///   --model                        Model alias or full name
///   --dangerously-skip-permissions Skip all permission prompts
///   --max-turns                    Agentic turn limit
///   --output-format                text | json | stream-json
pub async fn run_claude(
    input: &AgentInput,
    claude_path: &str,
    model: &str,
    agent_max_turns: u32,
    _session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mut args: Vec<String> = Vec::new();
    args.push("--print".to_string());
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    args.extend([
        "--dangerously-skip-permissions".to_string(),
        "--max-turns".to_string(),
        agent_max_turns.to_string(),
        "--output-format".to_string(),
        "text".to_string(),
    ]);
    // Pass the task as the print-mode prompt argument.
    args.push(full_prompt);
    let prompt_arg_index = args.len() - 1;

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    run_cli_agent(
        claude_path,
        &args_refs,
        Some(prompt_arg_index),
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
        None,
        &[],
    )
    .await
}
