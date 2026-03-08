use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db::{self, DbPool};
use crate::events::PipelineLogPayload;
use crate::models::PipelineStage;

/// Input passed to each agent invocation.
#[derive(Clone, Debug)]
pub struct AgentInput {
    pub prompt: String,
    pub context: Option<String>,
    pub workspace_path: String,
}

/// Output captured from agent execution.
#[derive(Clone, Debug)]
pub struct AgentOutput {
    pub raw_text: String,
    pub exit_code: i32,
}

/// Assembles a full prompt by concatenating the base prompt with optional
/// context sections.
pub fn build_full_prompt(input: &AgentInput) -> String {
    let mut parts = vec![input.prompt.clone()];
    if let Some(ref ctx) = input.context {
        parts.push(format!("\n\n--- Context ---\n{ctx}"));
    }
    parts.join("")
}

/// Spawns a CLI process, streams stdout/stderr line by line, and emits
/// `pipeline:log` events for each line. Returns the captured output and
/// exit code.
pub async fn run_cli_agent(
    binary: &str,
    args: &[&str],
    workspace_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(workspace_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {binary}: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Failed to capture stdout from {binary}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("Failed to capture stderr from {binary}"))?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let app_out = app.clone();
    let run_id_out = run_id.to_string();
    let stage_out = stage.clone();

    let app_err = app.clone();
    let run_id_err = run_id.to_string();
    let stage_err = stage.clone();

    let stage_str = serde_json::to_value(&stage)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{stage:?}"));

    let db_out = db.clone();
    let stage_str_out = stage_str.clone();

    let db_err = db.clone();
    let stage_str_err = stage_str;

    // Read stdout and stderr concurrently via separate tasks
    let stdout_handle = tokio::spawn(async move {
        let mut lines = Vec::new();
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            let _ = app_out.emit(
                "pipeline:log",
                PipelineLogPayload {
                    run_id: run_id_out.clone(),
                    stage: stage_out.clone(),
                    line: line.clone(),
                    stream: "stdout".to_string(),
                },
            );
            // Fire-and-forget log persistence
            let _ = db::logs::insert(&db_out, &run_id_out, &stage_str_out, &line, "stdout");
            lines.push(line);
        }
        lines
    });

    let stderr_handle = tokio::spawn(async move {
        let mut lines = Vec::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            let _ = app_err.emit(
                "pipeline:log",
                PipelineLogPayload {
                    run_id: run_id_err.clone(),
                    stage: stage_err.clone(),
                    line: line.clone(),
                    stream: "stderr".to_string(),
                },
            );
            let _ = db::logs::insert(&db_err, &run_id_err, &stage_str_err, &line, "stderr");
            lines.push(line);
        }
        lines
    });

    let stdout_lines = stdout_handle
        .await
        .map_err(|e| format!("stdout reader task failed: {e}"))?;
    let stderr_lines = stderr_handle
        .await
        .map_err(|e| format!("stderr reader task failed: {e}"))?;

    let mut all_output = stdout_lines.join("\n");
    if !stderr_lines.is_empty() {
        if !all_output.is_empty() {
            all_output.push('\n');
        }
        all_output.push_str(&stderr_lines.join("\n"));
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for {binary}: {e}"))?;

    Ok(AgentOutput {
        raw_text: all_output,
        exit_code: status.code().unwrap_or(-1),
    })
}
