use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use crate::events::PipelineErrorPayload;
use crate::models::*;
use crate::settings;

/// Shared application state, holding the pipeline cancellation flag and
/// the oneshot channel for delivering user answers to the blocked orchestrator.
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub answer_sender:
        Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
}

/// Validates a workspace directory and returns its git status.
#[tauri::command]
pub async fn select_workspace(path: String) -> Result<WorkspaceInfo, String> {
    let meta = std::fs::metadata(&path)
        .map_err(|e| format!("Cannot access path: {e}"))?;

    if !meta.is_dir() {
        return Err("Selected path is not a directory".to_string());
    }

    Ok(crate::git::workspace_info(&path))
}

/// Checks the health of all configured CLI tools.
#[tauri::command]
pub async fn validate_environment(settings: AppSettings) -> Result<CliHealth, String> {
    check_cli_health_inner(&settings).await
}

/// Starts the pipeline in a background task and returns immediately.
#[tauri::command]
pub async fn run_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    request: PipelineRequest,
) -> Result<(), String> {
    use tauri::Emitter;

    let loaded_settings = settings::load_settings();

    // Reset the cancellation flag
    state.cancel_flag.store(false, Ordering::SeqCst);
    let cancel_flag = state.cancel_flag.clone();
    let answer_sender = state.answer_sender.clone();

    // Spawn the pipeline as a background task so the command returns promptly
    let app_clone = app.clone();
    tokio::spawn(async move {
        let result = crate::orchestrator::run_pipeline(
            app_clone.clone(),
            request,
            loaded_settings,
            cancel_flag,
            answer_sender,
        )
        .await;

        if let Err(e) = result {
            let _ = app_clone.emit(
                "pipeline:error",
                PipelineErrorPayload {
                    run_id: String::new(),
                    stage: None,
                    message: e,
                },
            );
        }
    });

    Ok(())
}

/// Signals the running pipeline to cancel at the next stage boundary.
#[tauri::command]
pub async fn cancel_pipeline(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(())
}

/// Delivers the user's answer to a pending pipeline question.
#[tauri::command]
pub async fn answer_pipeline_question(
    state: State<'_, AppState>,
    answer: PipelineAnswer,
) -> Result<(), String> {
    let sender = {
        let mut lock = state.answer_sender.lock().await;
        lock.take()
    };

    match sender {
        Some(tx) => tx
            .send(answer)
            .map_err(|_| "Pipeline is no longer waiting for an answer".to_string()),
        None => Err("No pending question to answer".to_string()),
    }
}

/// Returns the current application settings (from disk or defaults).
#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    Ok(settings::load_settings())
}

/// Persists application settings to disk.
#[tauri::command]
pub async fn save_settings(new_settings: AppSettings) -> Result<(), String> {
    settings::save_settings_to_disk(&new_settings)
}

/// Checks whether each CLI binary is reachable.
#[tauri::command]
pub async fn check_cli_health(settings: AppSettings) -> Result<CliHealth, String> {
    check_cli_health_inner(&settings).await
}

/// Probes a single CLI binary using `which`.
async fn check_single_cli(path: &str) -> CliStatus {
    match tokio::process::Command::new("which")
        .arg(path)
        .output()
        .await
    {
        Ok(output) if output.status.success() => CliStatus {
            available: true,
            path: path.to_string(),
            error: None,
        },
        Ok(_) => CliStatus {
            available: false,
            path: path.to_string(),
            error: Some(format!("{path} not found in PATH")),
        },
        Err(e) => CliStatus {
            available: false,
            path: path.to_string(),
            error: Some(format!("Failed to check {path}: {e}")),
        },
    }
}

/// Shared implementation for CLI health checks.
async fn check_cli_health_inner(settings: &AppSettings) -> Result<CliHealth, String> {
    let (claude, codex, gemini) = tokio::join!(
        check_single_cli(&settings.claude_path),
        check_single_cli(&settings.codex_path),
        check_single_cli(&settings.gemini_path),
    );

    Ok(CliHealth {
        claude,
        codex,
        gemini,
    })
}
