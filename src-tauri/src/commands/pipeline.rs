use std::sync::atomic::Ordering;

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

    let loaded_settings = db::settings::get(&state.db)?;
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
