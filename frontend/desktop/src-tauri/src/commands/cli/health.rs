//! CLI health checking, version fetching, and update commands.

use crate::models::*;
use tauri::{AppHandle, Emitter};

use super::availability::check_binary_exists;
use super::util::run_npm;
use super::version::{build_cli_version_info, build_git_bash_version_info};
#[cfg(target_os = "windows")]
use super::git_bash;
#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};

/// Per-CLI health event payload.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CliHealthEvent {
    cli_name: String,
    status: CliStatus,
}

// ---------------------------------------------------------------------------
// Tauri commands — signatures must remain identical for the frontend contract.
// ---------------------------------------------------------------------------

/// Fire-and-forget: emits `cli_health_status` per CLI, then `cli_health_check_complete`.
#[tauri::command]
pub async fn check_cli_health(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let cli_paths = [
        ("claude", settings.claude_path.clone()),
        ("codex", settings.codex_path.clone()),
        ("gemini", settings.gemini_path.clone()),
        ("kimi", settings.kimi_path.clone()),
        ("opencode", settings.opencode_path.clone()),
    ];

    tokio::spawn(async move {
        // Windows: check bash availability first (fast — cached).
        let bash_missing = cfg!(target_os = "windows") && !check_binary_exists("bash").await;
        let app_complete = app.clone();

        let mut handles = Vec::with_capacity(5);
        for (cli_name, path) in cli_paths {
            let app_handle = app.clone();
            let cli_name = cli_name.to_string();
            handles.push(tokio::spawn(async move {
                let status = if bash_missing {
                    CliStatus {
                        available: false,
                        path: path.clone(),
                        error: Some("Git Bash is required on Windows to run agents".into()),
                    }
                } else {
                    check_single_cli(&path).await
                };
                let _ = app_handle.emit("cli_health_status", CliHealthEvent { cli_name, status });
            }));
        }
        for h in handles {
            let _ = h.await;
        }
        let _ = app_complete.emit("cli_health_check_complete", ());
    });

    Ok(())
}

/// Fire-and-forget: emits `cli_version_info` per CLI, then `cli_versions_check_complete`.
#[tauri::command]
pub async fn get_cli_versions(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let cli_specs: Vec<(String, &'static str, &'static str, &'static str)> = vec![
        (
            settings.claude_path.clone(),
            "Claude CLI",
            "claude",
            "@anthropic-ai/claude-code",
        ),
        (
            settings.codex_path.clone(),
            "Codex CLI",
            "codex",
            "@openai/codex",
        ),
        (
            settings.gemini_path.clone(),
            "Gemini CLI",
            "gemini",
            "@google/gemini-cli",
        ),
        (settings.kimi_path.clone(), "Kimi CLI", "kimi", "kimi-cli"),
        (
            settings.opencode_path.clone(),
            "OpenCode CLI",
            "opencode",
            "opencode-ai",
        ),
    ];

    tokio::spawn(async move {
        let app_complete = app.clone();
        let mut handles = Vec::with_capacity(6);

        for (path, display, cli_name, pkg) in cli_specs {
            let app_handle = app.clone();
            handles.push(tokio::spawn(async move {
                let info = build_cli_version_info(&path, display, cli_name, pkg).await;
                let _ = app_handle.emit("cli_version_info", &info);
            }));
        }

        // Git Bash (Windows only).
        if cfg!(target_os = "windows") {
            let app_handle = app.clone();
            handles.push(tokio::spawn(async move {
                let info = build_git_bash_version_info().await;
                let _ = app_handle.emit("cli_version_info", &info);
            }));
        }

        for h in handles {
            let _ = h.await;
        }
        let _ = app_complete.emit("cli_versions_check_complete", ());
    });

    Ok(())
}

/// Triggers an npm/pip update for the named CLI tool.
#[tauri::command]
pub async fn update_cli(app: AppHandle, cli_name: String) -> Result<String, String> {
    match cli_name.as_str() {
        "claude" => update_with_npm("@anthropic-ai/claude-code").await,
        "codex" => update_with_npm("@openai/codex").await,
        "gemini" => update_with_npm("@google/gemini-cli").await,
        "opencode" => update_with_npm("opencode-ai").await,
        "kimi" => update_kimi_cli().await,
        "gitBash" => update_git_bash(&app),
        _ => Err(format!("Unknown CLI: {cli_name}")),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Checks a single CLI binary and returns its health status.
async fn check_single_cli(path: &str) -> CliStatus {
    let available = check_binary_exists(path).await;
    CliStatus {
        available,
        path: path.to_string(),
        error: if available {
            None
        } else {
            Some(format!("{path} not found in PATH"))
        },
    }
}

/// Runs a full health check across all configured CLI tools.
///
/// Used internally by [`crate::commands::workspace::validate_environment`].
pub(crate) async fn check_cli_health_inner(settings: &AppSettings) -> Result<CliHealth, String> {
    let (mut claude, mut codex, mut gemini, mut kimi, mut opencode) = tokio::join!(
        check_single_cli(&settings.claude_path),
        check_single_cli(&settings.codex_path),
        check_single_cli(&settings.gemini_path),
        check_single_cli(&settings.kimi_path),
        check_single_cli(&settings.opencode_path),
    );
    if cfg!(target_os = "windows") && !check_binary_exists("bash").await {
        let required = Some("Git Bash is required on Windows to run agents".to_string());
        for status in [
            &mut claude,
            &mut codex,
            &mut gemini,
            &mut kimi,
            &mut opencode,
        ] {
            status.available = false;
            status.error = required.clone();
        }
    }
    Ok(CliHealth {
        claude,
        codex,
        gemini,
        kimi,
        opencode,
    })
}

/// Installs the latest version of an npm package globally.
async fn update_with_npm(npm_package: &str) -> Result<String, String> {
    let output = run_npm(&["install", "-g", &format!("{npm_package}@latest")]).await?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("Update failed: {stderr}"))
}

/// Updates the Kimi CLI — prefers `uv tool upgrade` when `uv` is available,
/// falling back to npm.
async fn update_kimi_cli() -> Result<String, String> {
    if check_binary_exists("uv").await {
        #[cfg(target_os = "windows")]
        let output = git_bash::run_binary("uv", &["tool", "upgrade", "kimi-cli", "--no-cache"], 20)
            .await
            .ok_or_else(|| "Failed to run uv via Git Bash".to_string())?;
        #[cfg(not(target_os = "windows"))]
        let output = {
            let mut cmd = tokio::process::Command::new("uv");
            cmd.args(["tool", "upgrade", "kimi-cli", "--no-cache"])
                .kill_on_drop(true);
            timeout(Duration::from_secs(20), cmd.output())
                .await
                .map_err(|_| "uv update timed out after 20 s".to_string())?
                .map_err(|e| format!("Failed to run uv: {e}"))?
        };

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Update failed: {stderr}"));
    }
    update_with_npm("kimi-cli").await
}

/// Opens the Git for Windows download page so the user can update manually.
fn update_git_bash(app: &AppHandle) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;
    let url = "https://git-scm.com/download/win";
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("Failed to open browser: {e}"))?;
    Ok("Opened Git download page — install the latest version to update.".to_string())
}
