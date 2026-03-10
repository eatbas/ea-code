use std::collections::HashMap;
use std::process::Output;

#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};

use crate::models::McpRuntimeStatus;

use super::parse;

#[cfg(target_os = "windows")]
use crate::commands::git_bash;

pub(super) async fn fetch_native_runtime_map(
    cli_path: &str,
) -> Result<HashMap<String, McpRuntimeStatus>, String> {
    let attempts: [&[&str]; 2] = [&["mcp", "list", "--json"], &["mcp", "list"]];
    let mut last_error = None::<String>;

    for args in attempts {
        let output = run_cli(cli_path, args, 25).await?;
        if !output.status.success() {
            last_error = Some(summarise_output(&output));
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(map) = parse::parse_runtime_map(stdout.as_ref()) {
            return Ok(map);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(map) = parse::parse_runtime_map(stderr.as_ref()) {
            return Ok(map);
        }

        last_error = Some(summarise_output(&output));
    }

    Err(format!(
        "Failed to read native MCP status from CLI output. {}",
        last_error.unwrap_or_else(|| "No usable output from CLI.".to_string())
    ))
}

pub(super) async fn run_cli(
    binary: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<Output, String> {
    #[cfg(target_os = "windows")]
    {
        return git_bash::run_binary(binary, args, timeout_secs)
            .await
            .ok_or_else(|| format!("Failed to run {binary} via Git Bash"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        timeout(
            Duration::from_secs(timeout_secs),
            tokio::process::Command::new(binary).args(args).output(),
        )
        .await
        .map_err(|_| format!("Timed out while running {binary}"))?
        .map_err(|e| format!("Failed to run {binary}: {e}"))
    }
}

pub(super) fn summarise_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stdout}\n{stderr}")
    };

    if combined.len() > 1000 {
        format!("{}...", &combined[..1000])
    } else if combined.is_empty() {
        "No CLI output.".to_string()
    } else {
        combined
    }
}
