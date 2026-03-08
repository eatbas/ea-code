//! Main orchestration pipeline loop.
//!
//! Runs: prompt enhance → plan → plan audit → generate → diff →
//!       review → fix → diff → judge → (loop if NOT COMPLETE)

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::{self, DbPool};
use crate::events::*;
use crate::models::*;

use super::helpers::*;
use super::iteration::run_iteration;
use super::prompts;
use super::run_setup::*;

// Re-export for helpers.rs which references IterationContext.
pub use super::run_setup::IterationContext;

/// Runs the full orchestration pipeline with v2.5.0 prompts.
pub async fn run_pipeline(
    app: AppHandle,
    request: PipelineRequest,
    settings: AppSettings,
    cancel_flag: Arc<AtomicBool>,
    answer_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: DbPool,
) -> Result<PipelineRun, String> {
    let run_id = Uuid::new_v4().to_string();
    let pipeline_start = Instant::now();

    let workspace_name = std::path::Path::new(&request.workspace_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| request.workspace_path.clone());
    let ws_info = crate::git::workspace_info(&request.workspace_path);
    let project_id = db::projects::upsert(
        &db, &request.workspace_path, &workspace_name,
        ws_info.is_git_repo, ws_info.branch.as_deref(),
    )?;

    let session_id = match request.session_id {
        Some(ref sid) if !sid.is_empty() => sid.clone(),
        _ => {
            let title = if request.prompt.chars().count() > 60 {
                format!("{}...", request.prompt.chars().take(60).collect::<String>())
            } else {
                request.prompt.clone()
            };
            let sid = Uuid::new_v4().to_string();
            db::sessions::create(&db, &sid, project_id, &title)?;
            sid
        }
    };

    db::runs::insert(&db, &run_id, &session_id, &request.prompt, settings.max_iterations as i32)?;

    let mut run = PipelineRun {
        id: run_id.clone(),
        status: PipelineStatus::Running,
        prompt: request.prompt.clone(),
        workspace_path: request.workspace_path.clone(),
        iterations: Vec::new(),
        current_iteration: 0,
        current_stage: None,
        max_iterations: settings.max_iterations,
        started_at: Some(epoch_millis()),
        completed_at: None,
        final_verdict: None,
        error: None,
    };

    let _ = app.emit(
        "pipeline:started",
        PipelineStartedPayload {
            run_id: run_id.clone(),
            prompt: request.prompt.clone(),
            workspace_path: request.workspace_path.clone(),
        },
    );

    let workspace_context =
        super::context_summary::build_workspace_context_summary(&request.workspace_path);
    emit_artifact(&app, &run_id, "workspace_context", &workspace_context, 0, &db);

    let mut previous_judge_output: Option<String> = None;
    let mut last_handoff: Option<prompts::IterationHandoff> = None;

    for iter_num in 1..=settings.max_iterations {
        if is_cancelled(&cancel_flag) {
            run.status = PipelineStatus::Cancelled;
            break;
        }

        let should_break = run_iteration(
            &app, &request, &settings, &cancel_flag, &answer_sender, &db,
            &run_id, &session_id, iter_num, &mut run,
            &mut previous_judge_output, &mut last_handoff, &workspace_context,
        )
        .await?;

        if should_break {
            break;
        }
    }

    if is_cancelled(&cancel_flag) {
        run.status = PipelineStatus::Cancelled;
    }

    run.current_stage = Some(PipelineStage::ExecutiveSummary);
    run_executive_summary(&app, &run_id, &run, &settings, &session_id, &db).await;

    let total_duration_ms = pipeline_start.elapsed().as_millis() as u64;
    run.completed_at = Some(epoch_millis());
    run.current_stage = None;

    persist_final_run(&db, &run, &session_id);
    emit_final_status(&app, &run, total_duration_ms);

    Ok(run)
}
