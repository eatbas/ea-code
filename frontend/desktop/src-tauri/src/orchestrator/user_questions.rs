//! User question helpers: pause the pipeline, emit a question event,
//! wait for a user answer (or cancellation / timeout), and persist Q&A.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::events::*;
use crate::models::RunEvent;
use crate::models::*;
use crate::storage::{self, runs};

use crate::orchestrator::helpers::{emit_stage, wait_for_cancel};

/// Pauses the pipeline and asks the user a question, persisting Q&A to event log.
pub async fn ask_user_question(
    app: &AppHandle,
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    question_text: String,
    agent_output: String,
    optional: bool,
    cancel_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
) -> Result<Option<PipelineAnswer>, String> {
    let question_id = Uuid::new_v4().to_string();

    let (tx, rx) = tokio::sync::oneshot::channel::<PipelineAnswer>();
    {
        let mut lock = answer_sender.lock().await;
        *lock = Some(tx);
    }

    emit_question_event(
        app,
        run_id,
        &question_id,
        stage,
        iteration,
        &question_text,
        &agent_output,
        optional,
    );
    emit_stage(app, run_id, stage, &StageStatus::WaitingForInput, iteration);

    tokio::select! {
        answer = rx => {
            handle_answer_result(answer, workspace_path, session_id, run_id, stage, iteration)
        }
        _ = wait_for_cancel(cancel_flag) => {
            let mut lock = answer_sender.lock().await;
            *lock = None;
            Ok(None)
        }
    }
}

/// Like `ask_user_question`, but auto-approves after `timeout_sec` seconds
/// if the user has not responded.
pub async fn ask_user_question_with_timeout(
    app: &AppHandle,
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    question_text: String,
    agent_output: String,
    optional: bool,
    cancel_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    timeout_sec: u64,
) -> Result<Option<PipelineAnswer>, String> {
    let question_id = Uuid::new_v4().to_string();

    let (tx, rx) = tokio::sync::oneshot::channel::<PipelineAnswer>();
    {
        let mut lock = answer_sender.lock().await;
        *lock = Some(tx);
    }

    emit_question_event(
        app,
        run_id,
        &question_id,
        stage,
        iteration,
        &question_text,
        &agent_output,
        optional,
    );
    emit_stage(app, run_id, stage, &StageStatus::WaitingForInput, iteration);

    let timeout_duration = std::time::Duration::from_secs(timeout_sec);

    tokio::select! {
        answer = rx => {
            handle_answer_result(answer, workspace_path, session_id, run_id, stage, iteration)
        }
        _ = tokio::time::sleep(timeout_duration) => {
            // Timeout — auto-approve by returning None (caller treats as approve).
            let mut lock = answer_sender.lock().await;
            *lock = None;
            // Log auto-approval as event
            let seq = runs::next_sequence(workspace_path, session_id, run_id).unwrap_or(1);
            let event = RunEvent::Question {
                v: 1,
                seq,
                ts: storage::now_rfc3339(),
                stage: stage.clone(),
                iteration,
                question: question_text,
                answer: "auto-approved (timeout)".to_string(),
                skipped: false,
            };
            let _ = runs::append_event(workspace_path, session_id, run_id, event);
            Ok(None)
        }
        _ = wait_for_cancel(cancel_flag) => {
            let mut lock = answer_sender.lock().await;
            *lock = None;
            Ok(None)
        }
    }
}

/// Emits the pipeline question event to the frontend.
fn emit_question_event(
    app: &AppHandle,
    run_id: &str,
    question_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    question_text: &str,
    agent_output: &str,
    optional: bool,
) {
    use tauri::Emitter;

    let _ = app.emit(
        EVENT_PIPELINE_QUESTION,
        PipelineQuestionPayload {
            run_id: run_id.to_string(),
            question_id: question_id.to_string(),
            stage: stage.clone(),
            iteration,
            question_text: question_text.to_string(),
            agent_output: agent_output.to_string(),
            optional,
        },
    );
}

/// Handles the result from a oneshot answer channel.
fn handle_answer_result(
    answer: Result<PipelineAnswer, tokio::sync::oneshot::error::RecvError>,
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
) -> Result<Option<PipelineAnswer>, String> {
    match answer {
        Ok(a) => {
            // Log Q&A to event log
            let seq = runs::next_sequence(workspace_path, session_id, run_id).unwrap_or(1);
            let event = RunEvent::Question {
                v: 1,
                seq,
                ts: storage::now_rfc3339(),
                stage: stage.clone(),
                iteration,
                question: if a.skipped {
                    "Question skipped".to_string()
                } else {
                    "User answered".to_string()
                },
                answer: if a.skipped {
                    "skipped".to_string()
                } else {
                    a.answer.clone()
                },
                skipped: a.skipped,
            };
            let _ = runs::append_event(workspace_path, session_id, run_id, event);
            Ok(Some(a))
        }
        Err(_) => Err("Answer channel dropped unexpectedly".to_string()),
    }
}
