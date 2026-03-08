use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::agents::{run_claude, run_codex, run_gemini, AgentInput, AgentOutput};
use crate::db::{self, DbPool};
use crate::events::*;
use crate::git;
use crate::models::*;

/// Returns the current time as epoch milliseconds (string).
fn epoch_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

/// Serialises a PipelineStage to its snake_case string for DB storage.
fn stage_to_str(stage: &PipelineStage) -> String {
    serde_json::to_value(stage)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{stage:?}"))
}

/// Dispatches to the appropriate agent runner based on the backend setting.
async fn dispatch_agent(
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    match backend {
        AgentBackend::Claude => {
            run_claude(
                input,
                &settings.claude_path,
                session_id,
                app,
                run_id,
                stage,
                db,
            )
            .await
        }
        AgentBackend::Codex => run_codex(input, &settings.codex_path, app, run_id, stage, db).await,
        AgentBackend::Gemini => {
            run_gemini(input, &settings.gemini_path, app, run_id, stage, db).await
        }
    }
}

/// Emits a stage status transition event.
fn emit_stage(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
) {
    let _ = app.emit(
        "pipeline:stage",
        PipelineStagePayload {
            run_id: run_id.to_string(),
            stage: stage.clone(),
            status: status.clone(),
            iteration,
        },
    );
}

/// Emits an artefact event and persists it to the database.
fn emit_artifact(
    app: &AppHandle,
    run_id: &str,
    kind: &str,
    content: &str,
    iteration: u32,
    db: &DbPool,
) {
    let _ = app.emit(
        "pipeline:artifact",
        PipelineArtifactPayload {
            run_id: run_id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            iteration,
        },
    );
    let _ = db::artifacts::insert(db, run_id, iteration as i32, kind, content);
}

/// Runs an agent stage: emits Running, executes, emits Completed/Failed,
/// persists the stage result, and returns it.
async fn execute_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    db: &DbPool,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    let stage_str = stage_to_str(&stage);

    match dispatch_agent(
        backend,
        input,
        settings,
        session_id,
        app,
        run_id,
        stage.clone(),
        db,
    )
    .await
    {
        Ok(output) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage(app, run_id, &stage, &StageStatus::Completed, iteration_num);
            let _ = db::runs::insert_stage(
                db,
                iteration_db_id,
                &stage_str,
                "completed",
                &output.raw_text,
                duration_ms as i32,
                None,
            );
            StageResult {
                stage,
                status: StageStatus::Completed,
                output: output.raw_text,
                duration_ms,
                error: None,
            }
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage(app, run_id, &stage, &StageStatus::Failed, iteration_num);
            let _ = db::runs::insert_stage(
                db,
                iteration_db_id,
                &stage_str,
                "failed",
                "",
                duration_ms as i32,
                Some(&e),
            );
            StageResult {
                stage,
                status: StageStatus::Failed,
                output: String::new(),
                duration_ms,
                error: Some(e),
            }
        }
    }
}

/// Captures a git diff and wraps it in a `StageResult`, persisting to DB.
fn execute_diff_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    workspace_path: &str,
    db: &DbPool,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);

    let diff = git::git_diff(workspace_path);
    let duration_ms = start.elapsed().as_millis() as u64;

    emit_stage(app, run_id, &stage, &StageStatus::Completed, iteration_num);
    emit_artifact(app, run_id, "diff", &diff, iteration_num, db);

    let stage_str = stage_to_str(&stage);
    let _ = db::runs::insert_stage(
        db,
        iteration_db_id,
        &stage_str,
        "completed",
        &diff,
        duration_ms as i32,
        None,
    );

    StageResult {
        stage,
        status: StageStatus::Completed,
        output: diff,
        duration_ms,
        error: None,
    }
}

/// Parses the judge verdict from raw output text.
fn parse_judge_verdict(output: &str) -> (JudgeVerdict, String) {
    let first_line = output.lines().next().unwrap_or("").trim();
    let reasoning = output.lines().skip(1).collect::<Vec<_>>().join("\n");
    let verdict = if first_line == "COMPLETE" {
        JudgeVerdict::Complete
    } else {
        JudgeVerdict::NotComplete
    };
    (verdict, reasoning)
}

fn is_cancelled(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

fn extract_question(output: &str) -> Option<String> {
    let start_tag = "[QUESTION]";
    let end_tag = "[/QUESTION]";
    if let Some(start) = output.find(start_tag) {
        if let Some(end) = output.find(end_tag) {
            let question = output[start + start_tag.len()..end].trim();
            if !question.is_empty() {
                return Some(question.to_string());
            }
        }
    }
    None
}

async fn wait_for_cancel(cancel_flag: &Arc<AtomicBool>) {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// Pauses the pipeline and asks the user a question, persisting Q&A to DB.
async fn ask_user_question(
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

    // Persist the question
    let _ = db::questions::insert(
        db,
        &question_id,
        run_id,
        &stage_str,
        iteration as i32,
        &question_text,
        &agent_output,
        optional,
    );

    let (tx, rx) = tokio::sync::oneshot::channel::<PipelineAnswer>();
    {
        let mut lock = answer_sender.lock().await;
        *lock = Some(tx);
    }

    let _ = app.emit(
        "pipeline:question",
        PipelineQuestionPayload {
            run_id: run_id.to_string(),
            question_id: question_id.clone(),
            stage: stage.clone(),
            iteration,
            question_text,
            agent_output,
            optional,
        },
    );

    emit_stage(app, run_id, stage, &StageStatus::WaitingForInput, iteration);

    tokio::select! {
        answer = rx => {
            match answer {
                Ok(a) => {
                    let _ = db::questions::record_answer(
                        db, &question_id,
                        if a.skipped { None } else { Some(&a.answer) },
                        a.skipped,
                    );
                    Ok(Some(a))
                }
                Err(_) => Err("Answer channel dropped unexpectedly".to_string()),
            }
        }
        _ = wait_for_cancel(cancel_flag) => {
            let mut lock = answer_sender.lock().await;
            *lock = None;
            Ok(None)
        }
    }
}

fn push_cancel_iteration(run: &mut PipelineRun, iter_num: u32, stages: Vec<StageResult>) {
    run.iterations.push(Iteration {
        number: iter_num,
        stages,
        verdict: None,
        judge_reasoning: None,
    });
    run.status = PipelineStatus::Cancelled;
}

fn build_prompt_enhancer_prompt(user_prompt: &str) -> String {
    format!(
        "You are a prompt enhancer for a multi-agent coding pipeline.\n\
Rewrite the user's request to be clear, precise, and implementation-ready.\n\
Preserve all constraints and intent.\n\
Return only the enhanced prompt text with no preamble.\n\n\
--- User Prompt ---\n{user_prompt}"
    )
}

fn normalise_enhanced_prompt(enhanced_output: &str, fallback_prompt: &str) -> String {
    let candidate = enhanced_output.trim();
    if candidate.is_empty() {
        fallback_prompt.to_string()
    } else {
        candidate.to_string()
    }
}

/// Runs the full orchestration pipeline:
///   prompt enhance → generate → diff → review → [ask user] → fix → diff → judge → loop
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

    // Register/touch project in DB
    let workspace_name = std::path::Path::new(&request.workspace_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| request.workspace_path.clone());
    let ws_info = git::workspace_info(&request.workspace_path);
    let project_id = db::projects::upsert(
        &db,
        &request.workspace_path,
        &workspace_name,
        ws_info.is_git_repo,
        ws_info.branch.as_deref(),
    )?;

    // Resolve or create session
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

    // Insert run record
    db::runs::insert(
        &db,
        &run_id,
        &session_id,
        &request.prompt,
        settings.max_iterations as i32,
    )?;

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

    for iter_num in 1..=settings.max_iterations {
        if is_cancelled(&cancel_flag) {
            run.status = PipelineStatus::Cancelled;
            break;
        }

        run.current_iteration = iter_num;
        let mut stages: Vec<StageResult> = Vec::new();
        let iteration_db_id = db::runs::insert_iteration(&db, &run_id, iter_num as i32)?;

        // --- 1. Prompt enhance ---
        let prompt_enhance_input = AgentInput {
            prompt: build_prompt_enhancer_prompt(&request.prompt),
            context: None,
            diff: None,
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::PromptEnhance);
        let prompt_enhance_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::PromptEnhance,
            &settings.prompt_enhancer_agent,
            &prompt_enhance_input,
            &settings,
            Some(session_id.as_str()),
            &db,
        )
        .await;
        let enhanced_prompt =
            normalise_enhanced_prompt(&prompt_enhance_result.output, &request.prompt);
        let prompt_enhance_failed = prompt_enhance_result.status == StageStatus::Failed;
        stages.push(prompt_enhance_result);
        if prompt_enhance_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if prompt_enhance_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Prompt Enhancer stage failed".to_string());
            }
            break;
        }

        // --- 2. Generate ---
        let gen_input = AgentInput {
            prompt: enhanced_prompt.clone(),
            context: None,
            diff: None,
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Generate);
        let gen_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::Generate,
            &settings.generator_agent,
            &gen_input,
            &settings,
            Some(session_id.as_str()),
            &db,
        )
        .await;
        let gen_output = gen_result.output.clone();
        let gen_failed = gen_result.status == StageStatus::Failed;
        stages.push(gen_result);
        if gen_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if gen_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Coder stage failed".to_string());
            }
            break;
        }

        if let Some(question) = extract_question(&gen_output) {
            let answer = ask_user_question(
                &app,
                &run_id,
                &PipelineStage::Generate,
                iter_num,
                question,
                gen_output.clone(),
                false,
                &cancel_flag,
                &answer_sender,
                &db,
            )
            .await?;
            if is_cancelled(&cancel_flag) {
                push_cancel_iteration(&mut run, iter_num, stages);
                break;
            }
            if let Some(ref a) = answer {
                if !a.skipped && !a.answer.is_empty() {
                    emit_stage(
                        &app,
                        &run_id,
                        &PipelineStage::Generate,
                        &StageStatus::Completed,
                        iter_num,
                    );
                }
            }
        }

        // --- 3. Diff after generate ---
        run.current_stage = Some(PipelineStage::DiffAfterGenerate);
        let diff1 = execute_diff_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::DiffAfterGenerate,
            &request.workspace_path,
            &db,
        );
        let diff1_output = diff1.output.clone();
        stages.push(diff1);
        if is_cancelled(&cancel_flag) {
            push_cancel_iteration(&mut run, iter_num, stages);
            break;
        }

        // --- 4. Review ---
        let review_input = AgentInput {
            prompt: enhanced_prompt.clone(),
            context: None,
            diff: Some(diff1_output.clone()),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Review);
        let review_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::Review,
            &settings.reviewer_agent,
            &review_input,
            &settings,
            Some(session_id.as_str()),
            &db,
        )
        .await;
        let review_output = review_result.output.clone();
        let review_failed = review_result.status == StageStatus::Failed;
        emit_artifact(&app, &run_id, "review", &review_output, iter_num, &db);
        stages.push(review_result);
        if review_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if review_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Code Reviewer / Auditor stage failed".to_string());
            }
            break;
        }

        // --- Ask user after Review ---
        let user_review_guidance = ask_user_question(
            &app,
            &run_id,
            &PipelineStage::Review,
            iter_num,
            "Review complete. Would you like to provide guidance for the fix stage?".to_string(),
            review_output.clone(),
            true,
            &cancel_flag,
            &answer_sender,
            &db,
        )
        .await?;
        if is_cancelled(&cancel_flag) {
            push_cancel_iteration(&mut run, iter_num, stages);
            break;
        }

        let fix_context = match user_review_guidance {
            Some(ref answer) if !answer.skipped && !answer.answer.is_empty() => {
                format!(
                    "{}\n\n--- User Guidance ---\n{}",
                    review_output, answer.answer
                )
            }
            _ => review_output.clone(),
        };

        // --- 5. Fix ---
        let fix_input = AgentInput {
            prompt: enhanced_prompt.clone(),
            context: Some(fix_context),
            diff: Some(diff1_output),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Fix);
        let fix_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::Fix,
            &settings.fixer_agent,
            &fix_input,
            &settings,
            Some(session_id.as_str()),
            &db,
        )
        .await;
        let fix_output = fix_result.output.clone();
        let fix_failed = fix_result.status == StageStatus::Failed;
        stages.push(fix_result);
        if fix_failed || is_cancelled(&cancel_flag) {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            if fix_failed {
                run.status = PipelineStatus::Failed;
                run.error = Some("Code Fixer stage failed".to_string());
            }
            break;
        }

        if let Some(question) = extract_question(&fix_output) {
            let answer = ask_user_question(
                &app,
                &run_id,
                &PipelineStage::Fix,
                iter_num,
                question,
                fix_output.clone(),
                false,
                &cancel_flag,
                &answer_sender,
                &db,
            )
            .await?;
            if is_cancelled(&cancel_flag) {
                push_cancel_iteration(&mut run, iter_num, stages);
                break;
            }
            let _ = answer;
        }

        // --- 6. Diff after fix ---
        run.current_stage = Some(PipelineStage::DiffAfterFix);
        let diff2 = execute_diff_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::DiffAfterFix,
            &request.workspace_path,
            &db,
        );
        let diff2_output = diff2.output.clone();
        stages.push(diff2);
        if is_cancelled(&cancel_flag) {
            push_cancel_iteration(&mut run, iter_num, stages);
            break;
        }

        // --- 7. Judge ---
        let judge_context = format!(
            "--- Review ---\n{}\n\n--- Fix ---\n{}",
            review_output, fix_output
        );
        let judge_input = AgentInput {
            prompt: enhanced_prompt,
            context: Some(judge_context),
            diff: Some(diff2_output),
            workspace_path: request.workspace_path.clone(),
        };
        run.current_stage = Some(PipelineStage::Judge);
        let judge_result = execute_agent_stage(
            &app,
            &run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::Judge,
            &settings.final_judge_agent,
            &judge_input,
            &settings,
            Some(session_id.as_str()),
            &db,
        )
        .await;
        let judge_output = judge_result.output.clone();
        let judge_failed = judge_result.status == StageStatus::Failed;
        emit_artifact(&app, &run_id, "judge", &judge_output, iter_num, &db);
        stages.push(judge_result);

        if judge_failed {
            run.iterations.push(Iteration {
                number: iter_num,
                stages,
                verdict: None,
                judge_reasoning: None,
            });
            run.status = PipelineStatus::Failed;
            run.error = Some("Judge stage failed".to_string());
            break;
        }

        let (verdict, reasoning) = parse_judge_verdict(&judge_output);
        let verdict_str = match &verdict {
            JudgeVerdict::Complete => "COMPLETE",
            JudgeVerdict::NotComplete => "NOT COMPLETE",
        };
        let _ = db::runs::update_iteration_verdict(
            &db,
            &run_id,
            iter_num as i32,
            Some(verdict_str),
            Some(&reasoning),
        );

        run.iterations.push(Iteration {
            number: iter_num,
            stages,
            verdict: Some(verdict.clone()),
            judge_reasoning: Some(reasoning),
        });

        if verdict == JudgeVerdict::Complete {
            run.final_verdict = Some(JudgeVerdict::Complete);
            run.status = PipelineStatus::Completed;
            break;
        }

        if iter_num == settings.max_iterations {
            run.final_verdict = Some(JudgeVerdict::NotComplete);
            run.status = PipelineStatus::Completed;
        }
    }

    if is_cancelled(&cancel_flag) {
        run.status = PipelineStatus::Cancelled;
    }

    let total_duration_ms = pipeline_start.elapsed().as_millis() as u64;
    run.completed_at = Some(epoch_millis());
    run.current_stage = None;

    // Persist final run status
    let status_str = match &run.status {
        PipelineStatus::Completed => "completed",
        PipelineStatus::Failed => "failed",
        PipelineStatus::Cancelled => "cancelled",
        _ => "completed",
    };
    let verdict_str = run.final_verdict.as_ref().map(|v| match v {
        JudgeVerdict::Complete => "COMPLETE",
        JudgeVerdict::NotComplete => "NOT COMPLETE",
    });
    let _ = db::runs::complete(&db, &run_id, status_str, verdict_str, run.error.as_deref());
    let _ = db::sessions::touch(&db, &session_id);

    match &run.status {
        PipelineStatus::Completed => {
            let _ = app.emit(
                "pipeline:completed",
                PipelineCompletedPayload {
                    run_id: run_id.clone(),
                    verdict: run
                        .final_verdict
                        .clone()
                        .unwrap_or(JudgeVerdict::NotComplete),
                    total_iterations: run.current_iteration,
                    duration_ms: total_duration_ms,
                },
            );
        }
        PipelineStatus::Failed => {
            let _ = app.emit(
                "pipeline:error",
                PipelineErrorPayload {
                    run_id: run_id.clone(),
                    stage: None,
                    message: run
                        .error
                        .clone()
                        .unwrap_or_else(|| "Unknown error".to_string()),
                },
            );
        }
        PipelineStatus::Cancelled => {
            let _ = app.emit(
                "pipeline:error",
                PipelineErrorPayload {
                    run_id: run_id.clone(),
                    stage: None,
                    message: "Pipeline cancelled by user".to_string(),
                },
            );
        }
        _ => {}
    }

    Ok(run)
}
