use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use crate::db::{self, DbPool};
use crate::events::PipelineErrorPayload;
use crate::models::*;

/// Shared application state, holding the pipeline cancellation flag,
/// the oneshot channel for delivering user answers, and the database pool.
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub answer_sender:
        Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    pub db: DbPool,
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

/// Returns the current application settings from the database.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    db::settings::get(&state.db)
}

/// Persists application settings to the database.
#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    new_settings: AppSettings,
) -> Result<(), String> {
    db::settings::update(&state.db, &new_settings)
}

// ── History / session query commands ────────────────────────────────────

/// Returns recently opened projects for the sidebar.
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<db::models::ProjectRow>, String> {
    db::projects::list_recent(&state.db, 20)
}

/// Returns session threads for a given project path.
#[tauri::command]
pub async fn list_sessions(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<Vec<db::models::SessionSummary>, String> {
    let project = db::projects::get_by_path(&state.db, &project_path)?
        .ok_or_else(|| "Project not found".to_string())?;
    db::sessions::list_for_project(&state.db, project.id, 50)
}

/// Returns full session detail with all runs for the ChatView.
#[tauri::command]
pub async fn get_session_detail(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<db::models::SessionDetail, String> {
    let session = db::sessions::get_by_id(&state.db, &session_id)?
        .ok_or_else(|| "Session not found".to_string())?;

    let project_path = db::sessions::get_project_path(&state.db, &session_id)?;

    let run_summaries = db::runs::list_for_session(&state.db, &session_id)?;
    let mut run_details = Vec::with_capacity(run_summaries.len());
    for rs in &run_summaries {
        run_details.push(db::runs::get_full(&state.db, &rs.id)?);
    }

    Ok(db::models::SessionDetail {
        id: session.id,
        title: session.title,
        project_path,
        created_at: session.created_at,
        updated_at: session.updated_at,
        runs: run_details,
    })
}

/// Creates a new session thread for a project.
#[tauri::command]
pub async fn create_session(
    state: State<'_, AppState>,
    project_path: String,
) -> Result<String, String> {
    let project = db::projects::get_by_path(&state.db, &project_path)?
        .ok_or_else(|| "Project not found".to_string())?;

    let session_id = uuid::Uuid::new_v4().to_string();
    db::sessions::create(&state.db, &session_id, project.id, "New Session")?;

    Ok(session_id)
}

/// Returns full detail for a single run.
#[tauri::command]
pub async fn get_run_detail(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<db::models::RunDetail, String> {
    db::runs::get_full(&state.db, &run_id)
}

/// Returns paginated logs for a run.
#[tauri::command]
pub async fn get_run_logs(
    state: State<'_, AppState>,
    run_id: String,
    offset: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<db::models::LogRow>, String> {
    db::logs::get_for_run(&state.db, &run_id, offset.unwrap_or(0), limit.unwrap_or(500))
}

/// Returns artefacts for a run.
#[tauri::command]
pub async fn get_run_artifacts(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<db::models::ArtifactRow>, String> {
    db::artifacts::get_for_run(&state.db, &run_id)
}

/// Deletes a session and all associated data.
#[tauri::command]
pub async fn delete_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    db::sessions::delete(&state.db, &session_id)
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
