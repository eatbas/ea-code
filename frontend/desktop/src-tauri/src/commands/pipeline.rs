use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, State};
use uuid::Uuid;

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
    if !request.direct_task {
        let missing = loaded_settings.missing_minimum_agents();
        if !missing.is_empty() {
            return Err(format!(
                "Cannot start pipeline. Go to Settings/Agents and set the minimum agent roles: {}.",
                missing.join(", ")
            ));
        }
    }
    let db = state.db.clone();
    let run_id = Uuid::new_v4().to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let pause_flag = Arc::new(AtomicBool::new(false));
    let answer_sender = Arc::new(tokio::sync::Mutex::new(None));

    {
        let mut flags = state.cancel_flags.lock().await;
        flags.insert(run_id.clone(), cancel_flag.clone());
    }
    {
        let mut flags = state.pause_flags.lock().await;
        flags.insert(run_id.clone(), pause_flag.clone());
    }
    {
        let mut senders = state.answer_senders.lock().await;
        senders.insert(run_id.clone(), answer_sender.clone());
    }
    let cancel_flags_registry = state.cancel_flags.clone();
    let pause_flags_registry = state.pause_flags.clone();
    let answer_senders_registry = state.answer_senders.clone();

    // Clone pool + retention setting so we can run cleanup after the pipeline finishes
    let cleanup_db = db.clone();
    let retention_days = loaded_settings.retention_days;

    // Spawn the pipeline as a background task so the command returns promptly
    let app_clone = app.clone();
    tokio::spawn(async move {
        let result = crate::orchestrator::run_pipeline(
            app_clone.clone(),
            run_id.clone(),
            request,
            loaded_settings,
            cancel_flag,
            pause_flag,
            answer_sender,
            db,
        )
        .await;

        // Best-effort retention cleanup after each run (skip VACUUM — too heavy)
        if retention_days > 0 {
            let _ = db::cleanup::cleanup_old_runs(&cleanup_db, retention_days as i32);
        }

        if let Err(e) = result {
            let _ = app_clone.emit(
                "pipeline:error",
                PipelineErrorPayload {
                    run_id: run_id.clone(),
                    stage: None,
                    message: e,
                },
            );
        }

        let mut flags = cancel_flags_registry.lock().await;
        flags.remove(&run_id);
        drop(flags);

        let mut flags = pause_flags_registry.lock().await;
        flags.remove(&run_id);
        drop(flags);

        let mut senders = answer_senders_registry.lock().await;
        senders.remove(&run_id);
    });

    Ok(())
}

/// Signals the running pipeline to cancel at the next stage boundary.
#[tauri::command]
pub async fn cancel_pipeline(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let cancel_flag = {
        let flags = state.cancel_flags.lock().await;
        flags.get(&run_id).cloned()
    };

    if let Some(flag) = cancel_flag {
        flag.store(true, Ordering::SeqCst);
    }
    let pause_flag = {
        let flags = state.pause_flags.lock().await;
        flags.get(&run_id).cloned()
    };
    if let Some(flag) = pause_flag {
        flag.store(false, Ordering::SeqCst);
    }

    let _ = db::run_status::cancel_run(&state.db, &run_id);
    Ok(())
}

#[tauri::command]
pub async fn pause_pipeline(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let pause_flag = {
        let flags = state.pause_flags.lock().await;
        flags.get(&run_id).cloned()
    };
    if let Some(flag) = pause_flag {
        flag.store(true, Ordering::SeqCst);
    }
    let _ = db::run_status::pause_run(&state.db, &run_id);
    Ok(())
}

#[tauri::command]
pub async fn resume_pipeline(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let pause_flag = {
        let flags = state.pause_flags.lock().await;
        flags.get(&run_id).cloned()
    };
    if let Some(flag) = pause_flag {
        flag.store(false, Ordering::SeqCst);
    }
    let _ = db::run_status::resume_run(&state.db, &run_id);
    Ok(())
}

/// Delivers the user's answer to a pending pipeline question.
#[tauri::command]
pub async fn answer_pipeline_question(
    state: State<'_, AppState>,
    answer: PipelineAnswer,
) -> Result<(), String> {
    let sender_slot = {
        let senders = state.answer_senders.lock().await;
        senders.get(&answer.run_id).cloned()
    };
    let sender = match sender_slot {
        Some(slot) => {
            let mut lock = slot.lock().await;
            lock.take()
        }
        None => None,
    };

    match sender {
        Some(tx) => tx
            .send(answer)
            .map_err(|_| "Pipeline is no longer waiting for an answer".to_string()),
        None => Err("No pending question to answer".to_string()),
    }
}
