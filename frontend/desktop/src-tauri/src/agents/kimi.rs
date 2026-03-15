use tauri::AppHandle;

use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs Kimi CLI in non-interactive print mode with stream-JSON output.
///
/// Flags per <https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html>:
///   --print   Non-interactive run mode
///   --output-format stream-json so live terminal can show incremental events
///   --model   Model override
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
) -> Result<AgentOutput, String> {
    let full_prompt = build_full_prompt(input);

    let requested_model = model.trim();
    let primary = run_kimi_once(
        input,
        kimi_path,
        requested_model,
        &full_prompt,
        app,
        run_id,
        stage.clone(),
    )
    .await?;

    // Compatibility fallback: some saved settings may still contain legacy
    // short aliases (`kimi-code`, `kimi-for-coding`) that resolve to no LLM.
    if !requested_model.is_empty()
        && !requested_model.contains('/')
        && primary.raw_text.contains("LLM not set")
    {
        let fallback_model = if requested_model == "kimi-code" {
            "kimi-code/kimi-for-coding".to_string()
        } else {
            format!("kimi-code/{requested_model}")
        };
        return run_kimi_once(
            input,
            kimi_path,
            &fallback_model,
            &full_prompt,
            app,
            run_id,
            stage,
        )
        .await;
    }

    Ok(primary)
}

async fn run_kimi_once(
    input: &AgentInput,
    kimi_path: &str,
    model: &str,
    full_prompt: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
) -> Result<AgentOutput, String> {
    let mut args = vec![
        "--print".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
    ];
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_cli_agent(
        kimi_path,
        &args_refs,
        None,
        &input.workspace_path,
        app,
        run_id,
        stage,
        Some(full_prompt),
        &[("PYTHONIOENCODING", "utf-8")],
    )
    .await?;

    Ok(AgentOutput {
        raw_text: extract_kimi_final_text(&output.raw_text),
    })
}

fn extract_kimi_final_text(stream_json_output: &str) -> String {
    let mut last_assistant_text: Option<String> = None;

    for line in stream_json_output.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("role").and_then(serde_json::Value::as_str) != Some("assistant") {
            continue;
        }
        let Some(content_parts) = value.get("content").and_then(serde_json::Value::as_array) else {
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

    last_assistant_text.unwrap_or_else(|| stream_json_output.trim().to_string())
}
