//! User question helpers: pause the pipeline, emit a question event,
//! wait for a user answer (or cancellation / timeout), and persist Q&A.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::{self, DbPool};
use crate::events::*;
use crate::models::*;

use super::helpers::{emit_stage, stage_to_str, wait_for_cancel};

/// Pauses the pipeline and asks the user a question, persisting Q&A to DB.
pub async fn ask_user_question(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    question_text: String,
    agent_output: String,
    optional: bool,
    cancel_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: &DbPool,
) -> Result<Option<PipelineAnswer>, String> {
    let question_id = Uuid::new_v4().to_string();
    let stage_str = stage_to_str(stage);

    let _ = db::questions::insert(
        db, &question_id, run_id, &stage_str,
        iteration as i32, &question_text, &agent_output, optional,
    );

    let (tx, rx) = tokio::sync::oneshot::channel::<PipelineAnswer>();
    {
        let mut lock = answer_sender.lock().await;
        *lock = Some(tx);
    }

    emit_question_event(app, run_id, &question_id, stage, iteration, &question_text, &agent_output, optional);
    emit_stage(app, run_id, stage, &StageStatus::WaitingForInput, iteration, db);

    tokio::select! {
        answer = rx => {
            handle_answer_result(answer, db, &question_id)
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
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    question_text: String,
    agent_output: String,
    optional: bool,
    cancel_flag: &Arc<AtomicBool>,
    answer_sender: &Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    db: &DbPool,
    timeout_sec: u64,
) -> Result<Option<PipelineAnswer>, String> {
    let question_id = Uuid::new_v4().to_string();
    let stage_str = stage_to_str(stage);

    let _ = db::questions::insert(
        db, &question_id, run_id, &stage_str,
        iteration as i32, &question_text, &agent_output, optional,
    );

    let (tx, rx) = tokio::sync::oneshot::channel::<PipelineAnswer>();
    {
        let mut lock = answer_sender.lock().await;
        *lock = Some(tx);
    }

    emit_question_event(app, run_id, &question_id, stage, iteration, &question_text, &agent_output, optional);
    emit_stage(app, run_id, stage, &StageStatus::WaitingForInput, iteration, db);

    let timeout_duration = std::time::Duration::from_secs(timeout_sec);

    tokio::select! {
        answer = rx => {
            handle_answer_result(answer, db, &question_id)
        }
        _ = tokio::time::sleep(timeout_duration) => {
            // Timeout — auto-approve by returning None (caller treats as approve).
            let mut lock = answer_sender.lock().await;
            *lock = None;
            let _ = db::questions::record_answer(db, &question_id, Some("auto-approved (timeout)"), false);
            Ok(None)
        }
        _ = wait_for_cancel(cancel_flag) => {
            let mut lock = answer_sender.lock().await;
            *lock = None;
            Ok(None)
        }
    }
}

/// Emits the `pipeline:question` event to the frontend.
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
        "pipeline:question",
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
    db: &DbPool,
    question_id: &str,
) -> Result<Option<PipelineAnswer>, String> {
    match answer {
        Ok(a) => {
            let _ = db::questions::record_answer(
                db, question_id,
                if a.skipped { None } else { Some(&a.answer) },
                a.skipped,
            );
            Ok(Some(a))
        }
        Err(_) => Err("Answer channel dropped unexpectedly".to_string()),
    }
}
