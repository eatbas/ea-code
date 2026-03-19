//! Main orchestration pipeline loop.
//!
//! Runs: prompt enhance → plan → plan audit → generate →
//!       review → fix → judge → (loop if NOT COMPLETE)

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::events::*;
use crate::models::*;
use crate::storage::{cleanup, messages, projects, runs, sessions};

use super::helpers::*;
use super::iteration::{run_iteration, IterationCarryover};
use super::run_setup::*;
use super::session_memory::{build_session_memory_context, merge_shared_context};

mod direct_task;

use direct_task::run_direct_task;

/// Runs the full orchestration pipeline with v2.5.0 prompts.
pub async fn run_pipeline(
    app: AppHandle,
    run_id: String,
    request: PipelineRequest,
    settings: AppSettings,
    cancel_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    answer_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
) -> Result<PipelineRun, String> {
    let pipeline_start = Instant::now();

    let workspace_name = std::path::Path::new(&request.workspace_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| request.workspace_path.clone());

    // Create or update project entry
    let project_entry = projects::create_project_entry(
        Uuid::new_v4().to_string(),
        request.workspace_path.clone(),
        workspace_name,
    );
    if let Err(e) = projects::add_project(&project_entry) {
        eprintln!("Warning: Failed to add project: {e}");
    }

    // Get or create session
    let session_id = match request.session_id {
        Some(ref sid) if !sid.is_empty() => sid.clone(),
        _ => {
            let title = if request.prompt.chars().count() > 60 {
                format!("{}...", request.prompt.chars().take(60).collect::<String>())
            } else {
                request.prompt.clone()
            };
            let sid = Uuid::new_v4().to_string();

            // Resolve the project_id — the project was just created/updated above
            let project_id = projects::find_by_path(&request.workspace_path)
                .map(|p| p.id)
                .unwrap_or_else(|| project_entry.id.clone());

            let session_meta = sessions::create_session_meta(
                sid.clone(),
                title,
                request.workspace_path.clone(),
                project_id,
            );
            if let Err(e) = sessions::create_session(&session_meta) {
                eprintln!("Warning: Failed to create session: {e}");
            }
            sid
        }
    };

    // Create run in storage
    if let Err(e) = runs::create_run(&run_id, &session_id, &request.prompt, settings.max_iterations, &request.workspace_path)
    {
        eprintln!("Warning: Failed to create run: {e}");
    }

    // Append user chat message to session log
    let user_msg = messages::user_message(&request.prompt, Some(run_id.clone()));
    if let Err(e) = messages::append_message(&session_id, &user_msg) {
        eprintln!("Warning: Failed to append user message: {e}");
    }

    // Touch session to update timestamp
    if let Err(e) = sessions::touch_session(&session_id, None, None, None) {
        eprintln!("Warning: Failed to touch session: {e}");
    }

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
        EVENT_PIPELINE_STARTED,
        PipelineStartedPayload {
            run_id: run_id.clone(),
            session_id: session_id.clone(),
            prompt: request.prompt.clone(),
            workspace_path: request.workspace_path.clone(),
            max_iterations: settings.max_iterations,
        },
    );

    let workspace_context =
        super::context_summary::build_workspace_context_summary(&request.workspace_path).await;
    let session_memory = build_session_memory_context(&session_id, Some(&run_id));
    let shared_context = merge_shared_context(&workspace_context, &session_memory);

    // Prime current stage immediately so session-detail polling can show
    // a live progress indicator before the first stage body starts.
    let bootstrap_stage = if request.direct_task {
        PipelineStage::DirectTask
    } else {
        PipelineStage::PromptEnhance
    };
    run.current_iteration = 1;
    run.current_stage = Some(bootstrap_stage.clone());
    emit_stage(&app, &run_id, &bootstrap_stage, &StageStatus::Running, 1);

    if request.direct_task {
        run_direct_task(
            &app,
            &request,
            &settings,
            &cancel_flag,
            &pause_flag,
            &run_id,
            &session_id,
            &shared_context,
            &mut run,
        )
        .await?;
    } else {
        let mut carry = IterationCarryover::new();

        for iter_num in 1..=settings.max_iterations {
            if wait_if_paused(&pause_flag, &cancel_flag).await {
                run.status = PipelineStatus::Cancelled;
                break;
            }
            if is_cancelled(&cancel_flag) {
                run.status = PipelineStatus::Cancelled;
                break;
            }

            let should_break = run_iteration(
                &app,
                &request,
                &settings,
                &cancel_flag,
                &pause_flag,
                &answer_sender,
                &run_id,
                &session_id,
                iter_num,
                &mut run,
                &mut carry,
                &shared_context,
            )
            .await?;

            if should_break {
                break;
            }
        }

        if is_cancelled(&cancel_flag) {
            run.status = PipelineStatus::Cancelled;
        }
    }

    // Keep run-level continuity data up to date for future runs.
    if !matches!(run.status, PipelineStatus::Cancelled) {
        run.current_stage = Some(PipelineStage::ExecutiveSummary);
        run_executive_summary(&app, &run_id, &run, &settings, &session_id).await;
    }

    // Append assistant chat message with executive summary or cancellation notice
    let assistant_content = if matches!(run.status, PipelineStatus::Cancelled) {
        "Pipeline cancelled by user.".to_string()
    } else {
        runs::read_summary(&run_id)
            .ok()
            .and_then(|s| s.executive_summary)
            .unwrap_or_else(|| {
                let status_label = if run.status == PipelineStatus::Completed {
                    "completed"
                } else {
                    "ended"
                };
                format!("Pipeline {status_label}.")
            })
    };
    let assistant_msg = messages::assistant_message(&assistant_content, Some(run_id.clone()));
    if let Err(e) = messages::append_message(&session_id, &assistant_msg) {
        eprintln!("Warning: Failed to append assistant message: {e}");
    }

    let total_duration_ms = pipeline_start.elapsed().as_millis() as u64;
    run.completed_at = Some(epoch_millis());
    run.current_stage = None;

    persist_final_run(&run, &session_id);
    emit_final_status(&app, &run, total_duration_ms);
    run_retention_cleanup(settings.retention_days);

    Ok(run)
}

fn run_retention_cleanup(retention_days: u32) {
    if retention_days == 0 {
        return;
    }
    match cleanup::cleanup_old_runs(retention_days) {
        Ok(()) => {}
        Err(err) => eprintln!("Warning: post-run retention cleanup failed: {err}"),
    }
}
