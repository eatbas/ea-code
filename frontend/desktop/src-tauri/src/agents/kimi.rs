use tauri::AppHandle;

use crate::db::DbPool;
use crate::models::PipelineStage;

use super::base::{build_full_prompt, run_cli_agent, AgentInput, AgentOutput};

/// Runs Kimi CLI in non-interactive quiet mode.
///
/// Flags per <https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html>:
///   --quiet   Shortcut for --print --output-format text --final-message-only
///   --model   Model override
///   --prompt  Non-interactive prompt input
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

    let requested_model = model.trim();
    let primary = run_kimi_once(
        input,
        kimi_path,
        requested_model,
        &full_prompt,
        app,
        run_id,
        stage.clone(),
        db,
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
            db,
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
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let mut args = vec!["--quiet".to_string()];
    if !model.is_empty() {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    args.push("--prompt".to_string());
    args.push(full_prompt.to_string());
    let prompt_arg_index = args.len() - 1;
    let args_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli_agent(
        kimi_path,
        &args_refs,
        Some(prompt_arg_index),
        &input.workspace_path,
        app,
        run_id,
        stage,
        db,
        None,
        &[("PYTHONIOENCODING", "utf-8")],
    )
    .await
}
