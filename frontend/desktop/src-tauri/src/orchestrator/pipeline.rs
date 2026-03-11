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

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::events::*;
use crate::models::*;

use super::helpers::*;
use super::iteration::run_iteration;
use super::prompts;
use super::run_setup::*;
use super::session_memory::{build_session_memory_context, merge_shared_context};


/// Runs the full orchestration pipeline with v2.5.0 prompts.
pub async fn run_pipeline(
    app: AppHandle,
    run_id: String,
    request: PipelineRequest,
    settings: AppSettings,
    cancel_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    answer_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: DbPool,
) -> Result<PipelineRun, String> {
    let pipeline_start = Instant::now();

    let workspace_name = std::path::Path::new(&request.workspace_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| request.workspace_path.clone());
    let ws_info = crate::git::workspace_info(&request.workspace_path).await;
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
    let _ = db::sessions::touch(&db, &session_id);

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
            session_id: session_id.clone(),
            prompt: request.prompt.clone(),
            workspace_path: request.workspace_path.clone(),
        },
    );

    let workspace_context =
        super::context_summary::build_workspace_context_summary(&request.workspace_path).await;
    let session_memory = build_session_memory_context(&db, &session_id, Some(&run_id));
    let shared_context = merge_shared_context(&workspace_context, &session_memory);
    emit_artifact(&app, &run_id, "workspace_context", &workspace_context, 0, &db);
    if !session_memory.trim().is_empty() {
        emit_artifact(&app, &run_id, "session_memory", &session_memory, 0, &db);
    }

    // Prime current stage immediately so session-detail polling can show
    // a live progress indicator before the first stage body starts.
    let bootstrap_stage = if request.direct_task {
        PipelineStage::DirectTask
    } else {
        PipelineStage::PromptEnhance
    };
    run.current_iteration = 1;
    run.current_stage = Some(bootstrap_stage.clone());
    emit_stage(
        &app,
        &run_id,
        &bootstrap_stage,
        &StageStatus::Running,
        1,
        &db,
    );

    if request.direct_task {
        run_direct_task(
            &app,
            &request,
            &settings,
            &cancel_flag,
            &pause_flag,
            &db,
            &run_id,
            &session_id,
            &shared_context,
            &mut run,
        )
        .await?;
    } else {
        let mut previous_judge_output: Option<String> = None;
        let mut last_handoff: Option<prompts::IterationHandoff> = None;

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
                &app, &request, &settings, &cancel_flag, &pause_flag, &answer_sender, &db,
                &run_id, &session_id, iter_num, &mut run,
                &mut previous_judge_output, &mut last_handoff, &shared_context,
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
        run_executive_summary(&app, &run_id, &run, &settings, &session_id, &db).await;
    }

    let total_duration_ms = pipeline_start.elapsed().as_millis() as u64;
    run.completed_at = Some(epoch_millis());
    run.current_stage = None;

    persist_final_run(&db, &run, &session_id);
    emit_final_status(&app, &run, total_duration_ms);

    Ok(run)
}

/// Executes a single agent call directly, bypassing the full pipeline.
async fn run_direct_task(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &Arc<AtomicBool>,
    pause_flag: &Arc<AtomicBool>,
    db: &DbPool,
    run_id: &str,
    session_id: &str,
    shared_context: &str,
    run: &mut PipelineRun,
) -> Result<(), String> {
    let backend = request
        .direct_task_agent
        .as_ref()
        .ok_or_else(|| "Direct task mode requires an agent backend".to_string())?;
    let model = request.direct_task_model.as_deref().unwrap_or("");

    let iteration_db_id = db::runs::insert_iteration(db, run_id, 1)?;
    run.current_iteration = 1;
    run.current_stage = Some(PipelineStage::DirectTask);

    let input = AgentInput {
        prompt: request.prompt.clone(),
        context: if shared_context.trim().is_empty() {
            None
        } else {
            Some(shared_context.to_string())
        },
        workspace_path: request.workspace_path.clone(),
    };

    let start = Instant::now();
    emit_stage(app, run_id, &PipelineStage::DirectTask, &StageStatus::Running, 1, db);

    if wait_if_paused(pause_flag, cancel_flag).await {
        push_cancel_iteration(run, 1, Vec::new());
        return Ok(());
    }
    if is_cancelled(cancel_flag) {
        push_cancel_iteration(run, 1, Vec::new());
        return Ok(());
    }

    let result = dispatch_agent(
        backend,
        model,
        &input,
        settings,
        Some(session_id),
        app,
        run_id,
        PipelineStage::DirectTask,
        db,
    )
    .await;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(out) => {
            emit_stage_with_duration(app, run_id, &PipelineStage::DirectTask, &StageStatus::Completed, 1, Some(duration_ms), db);
            // Emit the raw CLI output as a "result" artefact so the frontend can parse it
            emit_artifact(app, run_id, "result", &out.raw_text, 1, db);
            let _ = db::runs::insert_stage(
                db, iteration_db_id, "direct_task", "completed",
                &out.raw_text, duration_ms as i32, None,
            );
            run.iterations.push(Iteration {
                number: 1,
                stages: vec![StageResult {
                    stage: PipelineStage::DirectTask,
                    status: StageStatus::Completed,
                    output: out.raw_text,
                    duration_ms,
                    error: None,
                }],
                verdict: Some(JudgeVerdict::Complete),
                judge_reasoning: None,
            });
            run.status = PipelineStatus::Completed;
            run.final_verdict = Some(JudgeVerdict::Complete);
        }
        Err(e) => {
            emit_stage_with_duration(app, run_id, &PipelineStage::DirectTask, &StageStatus::Failed, 1, Some(duration_ms), db);
            let _ = db::runs::insert_stage(
                db, iteration_db_id, "direct_task", "failed",
                "", duration_ms as i32, Some(&e),
            );
            run.iterations.push(Iteration {
                number: 1,
                stages: vec![StageResult {
                    stage: PipelineStage::DirectTask,
                    status: StageStatus::Failed,
                    output: String::new(),
                    duration_ms,
                    error: Some(e.clone()),
                }],
                verdict: None,
                judge_reasoning: None,
            });
            run.status = PipelineStatus::Failed;
            run.error = Some(e);
        }
    }

    Ok(())
}
