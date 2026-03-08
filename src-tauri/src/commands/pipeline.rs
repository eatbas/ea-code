use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, State};

use crate::db;
use crate::events::PipelineErrorPayload;
use crate::models::*;

use super::AppState;

/// Starts the pipeline in a background task and returns immediately.
#[tauri::command]
pub async fn run_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    request: PipelineRequest,
) -> Result<(), String> {
    use tauri::Emitter;

    let loaded_settings = db::settings::get_merged_for_workspace(&state.db, &request.workspace_path)?;
    run_startup_cli_updates(&loaded_settings).await?;
    let db = state.db.clone();

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
            db,
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

async fn run_startup_cli_updates(settings: &AppSettings) -> Result<(), String> {
    if !settings.update_cli_on_run {
        return Ok(());
    }

    let timeout_ms = settings.cli_update_timeout_ms.max(1_000);
    let mut failures = Vec::new();

    for cli_name in ["claude", "codex", "gemini", "kimi", "opencode"] {
        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            super::cli::update_cli(cli_name.to_string()),
        )
        .await;

        match result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => failures.push(format!("{cli_name}: {e}")),
            Err(_) => failures.push(format!("{cli_name}: update timed out after {timeout_ms}ms")),
        }
    }

    if failures.is_empty() {
        return Ok(());
    }

    let message = format!("CLI startup update failures:\n{}", failures.join("\n"));
    if settings.fail_on_cli_update_error {
        Err(message)
    } else {
        eprintln!("{message}");
        Ok(())
    }
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
