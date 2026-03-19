use tauri::AppHandle;

use crate::models::{PipelineStage, StageExecutionIntent};
use crate::orchestrator::helpers::looks_like_output_file_instruction;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs Claude Code in non-interactive stream-JSON mode.
///
/// Flags per <https://code.claude.com/docs/en/cli-reference>:
///   --print                        Non-interactive execution
///   --model                        Model alias or full name
///   --dangerously-skip-permissions Skip all permission prompts
///   --max-turns                    Agentic turn limit
///   --verbose                      Required for stream-json mode
///   --output-format                stream-json for live terminal output
pub async fn run_claude(
    input: &AgentInput,
    claude_path: &str,
    model: &str,
    agent_max_turns: u32,
    _session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    intent: StageExecutionIntent,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mut args: Vec<String> = Vec::new();
    args.push("--print".to_string());
    args.push("--verbose".to_string());
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    // All pipeline stages are non-interactive, so permission prompts cannot
    // be answered. Skip them for every intent.
    args.push("--dangerously-skip-permissions".to_string());

    // Text-intent stages (planners, reviewers, judge) must not modify files.
    // Remove write tools entirely so the model cannot code even if it tries.
    // Also disallow Agent to prevent Opus from spawning sub-agents that
    // inherit the same restrictions and get stuck in a loop.
    if matches!(intent, StageExecutionIntent::Text) {
        for tool in &["Edit", "Write", "Bash", "NotebookEdit", "Agent"] {
            args.push("--disallowedTools".to_string());
            args.push(tool.to_string());
        }
    }
    args.extend([
        "--max-turns".to_string(),
        agent_max_turns.to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
    ]);

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let output = run_cli_agent(
        claude_path,
        &args_refs,
        None, // prompt is piped via stdin
        &input.workspace_path,
        app,
        run_id,
        stage,
        Some(&full_prompt),
        &[],
    )
    .await?;

    Ok(AgentOutput {
        raw_text: extract_claude_final_text(&output.raw_text),
    })
}

fn extract_claude_final_text(stream_json_output: &str) -> String {
    let mut last_result_text: Option<String> = None;
    let mut last_assistant_text: Option<String> = None;

    for line in stream_json_output.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        if value.get("type").and_then(serde_json::Value::as_str) == Some("result") {
            if let Some(text) = value
                .get("result")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                last_result_text = Some(text.to_string());
            }
            continue;
        }

        if value.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(content_parts) = value
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };

        let text_parts = content_parts
            .iter()
            .filter(|part| part.get("type").and_then(serde_json::Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        if !text_parts.is_empty() {
            last_assistant_text = Some(text_parts.join("\n\n"));
        }
    }

    // Discard texts that are just file-write instructions (e.g. Claude Code
    // telling the model to write to PLAN.md). These are not real output.
    let result_text = last_result_text.filter(|t| !looks_like_output_file_instruction(t));
    let assistant_text = last_assistant_text.filter(|t| !looks_like_output_file_instruction(t));

    // Prefer whichever text is more substantial. The `result.result` field
    // from Claude CLI can be a brief summary while the last assistant message
    // contains the full detailed response (e.g. a complete plan).
    match (result_text, assistant_text) {
        (Some(result), Some(assistant)) => {
            if assistant.len() > result.len() {
                assistant
            } else {
                result
            }
        }
        (Some(result), None) => result,
        (None, Some(assistant)) => assistant,
        (None, None) => stream_json_output.trim().to_string(),
    }
}

