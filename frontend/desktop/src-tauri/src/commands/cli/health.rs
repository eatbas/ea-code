//! CLI health checking, version fetching, and update commands.

use crate::commands::emitter::spawn_joined_task_emits;
use crate::models::{AgentBackend, AppSettings, CliHealth, CliStatus};
use tauri::{AppHandle, Emitter};

use super::availability::check_binary_exists;
#[cfg(target_os = "windows")]
use super::git_bash;
use super::util::run_npm;
use super::version::{build_cli_version_info, build_git_bash_version_info};
#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};

pub const EVENT_CLI_HEALTH_STATUS: &str = "cli_health_status";
pub const EVENT_CLI_HEALTH_COMPLETE: &str = "cli_health_check_complete";
pub const EVENT_CLI_VERSION_INFO: &str = "cli_version_info";
pub const EVENT_CLI_VERSIONS_COMPLETE: &str = "cli_versions_check_complete";

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CliHealthEvent {
    cli_name: String,
    status: CliStatus,
}

#[tauri::command]
pub async fn check_cli_health(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let bash_missing = cfg!(target_os = "windows") && !check_binary_exists("bash").await;
    let mut handles = Vec::with_capacity(AgentBackend::MANAGED.len());

    for backend in AgentBackend::MANAGED {
        let app_handle = app.clone();
        let cli_name = backend.as_str().to_string();
        let path = settings.path_for_backend(backend).to_string();
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
            let _ = app_handle.emit(EVENT_CLI_HEALTH_STATUS, CliHealthEvent { cli_name, status });
        }));
    }

    spawn_joined_task_emits(app, EVENT_CLI_HEALTH_COMPLETE, handles);
    Ok(())
}

#[tauri::command]
pub async fn get_cli_versions(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let mut handles =
        Vec::with_capacity(AgentBackend::MANAGED.len() + usize::from(cfg!(target_os = "windows")));

    for backend in AgentBackend::MANAGED {
        let app_handle = app.clone();
        let path = settings.path_for_backend(backend).to_string();
        handles.push(tokio::spawn(async move {
            let info = build_cli_version_info(&path, backend).await;
            let _ = app_handle.emit(EVENT_CLI_VERSION_INFO, &info);
        }));
    }

    if cfg!(target_os = "windows") {
        let app_handle = app.clone();
        handles.push(tokio::spawn(async move {
            let info = build_git_bash_version_info().await;
            let _ = app_handle.emit(EVENT_CLI_VERSION_INFO, &info);
        }));
    }

    spawn_joined_task_emits(app, EVENT_CLI_VERSIONS_COMPLETE, handles);
    Ok(())
}

#[tauri::command]
pub async fn update_cli(app: AppHandle, cli_name: String) -> Result<String, String> {
    match cli_name.as_str() {
        "claude" | "codex" | "gemini" | "opencode" => {
            let backend = AgentBackend::from_cli_name(&cli_name)
                .ok_or_else(|| format!("Unknown CLI: {cli_name}"))?;
            let package_name = backend
                .package_name()
                .ok_or_else(|| format!("Unknown CLI: {cli_name}"))?;
            update_with_npm(package_name).await
        }
        "kimi" => update_kimi_cli().await,
        "gitBash" => update_git_bash(&app),
        _ => Err(format!("Unknown CLI: {cli_name}")),
    }
}

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

async fn update_with_npm(npm_package: &str) -> Result<String, String> {
    let output = run_npm(&["install", "-g", &format!("{npm_package}@latest")]).await?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("Update failed: {stderr}"))
}

async fn update_kimi_cli() -> Result<String, String> {
    if check_binary_exists("uv").await {
        #[cfg(target_os = "windows")]
        let output = git_bash::run_binary("uv", &["tool", "upgrade", "kimi-cli", "--no-cache"], 20)
            .await
            .ok_or_else(|| "Failed to run uv via Git Bash".to_string())?;
        #[cfg(not(target_os = "windows"))]
        let output = {
            let mut command = tokio::process::Command::new("uv");
            command
                .args(["tool", "upgrade", "kimi-cli", "--no-cache"])
                .kill_on_drop(true);
            timeout(Duration::from_secs(20), command.output())
                .await
                .map_err(|_| "uv update timed out after 20 s".to_string())?
                .map_err(|error| format!("Failed to run uv: {error}"))?
        };

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Update failed: {stderr}"));
    }
    update_with_npm("kimi-cli").await
}

fn update_git_bash(app: &AppHandle) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;
    let url = "https://git-scm.com/download/win";
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| format!("Failed to open browser: {error}"))?;
    Ok("Opened Git download page — install the latest version to update.".to_string())
}
