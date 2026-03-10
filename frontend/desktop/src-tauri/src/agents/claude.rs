use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

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
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let mut args: Vec<String> = Vec::new();
    args.push("--print".to_string());
    args.push("--verbose".to_string());
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    args.extend([
        "--dangerously-skip-permissions".to_string(),
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
        db,
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

    last_result_text
        .or(last_assistant_text)
        .unwrap_or_else(|| stream_json_output.trim().to_string())
}
