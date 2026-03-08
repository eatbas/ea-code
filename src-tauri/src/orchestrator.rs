use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::agents::{
    run_claude, run_codex, run_gemini, run_kimi, run_opencode, AgentInput, AgentOutput,
};
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

#[derive(Clone, Debug)]
struct IterationContext {
    enhanced_prompt: String,
    planner_plan: Option<String>,
    audit_verdict: Option<String>,
    audit_reasoning: Option<String>,
    audited_plan: Option<String>,
    review_output: Option<String>,
    review_user_guidance: Option<String>,
    fix_output: Option<String>,
    judge_output: Option<String>,
    generate_question: Option<String>,
    generate_answer: Option<String>,
    fix_question: Option<String>,
    fix_answer: Option<String>,
}

impl IterationContext {
    fn new(original_prompt: String) -> Self {
        Self {
            enhanced_prompt: original_prompt,
            planner_plan: None,
            audit_verdict: None,
            audit_reasoning: None,
            audited_plan: None,
            review_output: None,
            review_user_guidance: None,
            fix_output: None,
            judge_output: None,
            generate_question: None,
            generate_answer: None,
            fix_question: None,
            fix_answer: None,
        }
    }

    fn selected_plan(&self) -> Option<&str> {
        self.audited_plan
            .as_deref()
            .or(self.planner_plan.as_deref())
    }
}

fn persist_iteration_context(
    db: &DbPool,
    run_id: &str,
    iteration: u32,
    context: &IterationContext,
) {
    let patch = db::runs::IterationContextPatch {
        enhanced_prompt: Some(context.enhanced_prompt.as_str()),
        planner_plan: context.planner_plan.as_deref(),
        audit_verdict: context.audit_verdict.as_deref(),
        audit_reasoning: context.audit_reasoning.as_deref(),
        audited_plan: context.audited_plan.as_deref(),
        review_output: context.review_output.as_deref(),
        review_user_guidance: context.review_user_guidance.as_deref(),
        fix_output: context.fix_output.as_deref(),
        judge_output: context.judge_output.as_deref(),
        generate_question: context.generate_question.as_deref(),
        generate_answer: context.generate_answer.as_deref(),
        fix_question: context.fix_question.as_deref(),
        fix_answer: context.fix_answer.as_deref(),
    };
    let _ = db::runs::update_iteration_context(db, run_id, iteration as i32, &patch);
}

fn stage_context_with_original(original_prompt: &str, extra: Option<String>) -> String {
    match extra {
        Some(text) if !text.trim().is_empty() => {
            format!("--- Original Prompt ---\n{original_prompt}\n\n{text}")
        }
        _ => format!("--- Original Prompt ---\n{original_prompt}"),
    }
}

/// Dispatches to the appropriate agent runner based on the backend setting.
async fn dispatch_agent(
    backend: &AgentBackend,
    model: &str,
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
                model,
                session_id,
                app,
                run_id,
                stage,
                db,
            )
            .await
        }
        AgentBackend::Codex => {
            run_codex(
                input,
                &settings.codex_path,
                model,
                app,
                run_id,
                stage,
                db,
            )
            .await
        }
        AgentBackend::Gemini => {
            run_gemini(
                input,
                &settings.gemini_path,
                model,
                app,
                run_id,
                stage,
                db,
            )
            .await
        }
        AgentBackend::Kimi => {
            run_kimi(input, &settings.kimi_path, model, app, run_id, stage, db).await
        }
        AgentBackend::OpenCode => {
            run_opencode(
                input,
                &settings.opencode_path,
                model,
                app,
                run_id,
                stage,
                db,
            )
            .await
        }
    }
}

/// Resolves the model to use for a given pipeline stage from the per-stage settings.
/// Falls back to the first enabled model for the stage's CLI backend.
fn resolve_stage_model(stage: &PipelineStage, settings: &AppSettings) -> String {
    match stage {
        PipelineStage::PromptEnhance => settings.prompt_enhancer_model.clone(),
        PipelineStage::Plan => settings
            .planner_model
            .clone()
            .unwrap_or_else(|| first_enabled_model_for_backend(settings.planner_agent.as_ref(), settings)),
        PipelineStage::PlanAudit => settings
            .plan_auditor_model
            .clone()
            .unwrap_or_else(|| first_enabled_model_for_backend(settings.plan_auditor_agent.as_ref(), settings)),
        PipelineStage::Generate => settings.generator_model.clone(),
        PipelineStage::Review => settings.reviewer_model.clone(),
        PipelineStage::Fix => settings.fixer_model.clone(),
        PipelineStage::Judge => settings.final_judge_model.clone(),
        PipelineStage::ExecutiveSummary => settings.executive_summary_model.clone(),
        // Diff stages don't invoke an agent; return a placeholder.
        PipelineStage::DiffAfterGenerate | PipelineStage::DiffAfterFix => String::new(),
    }
}

/// Returns the first enabled model for a given backend from the comma-separated list.
fn first_enabled_model_for_backend(backend: Option<&AgentBackend>, settings: &AppSettings) -> String {
    let csv = match backend {
        Some(AgentBackend::Claude) => &settings.claude_model,
        Some(AgentBackend::Codex) => &settings.codex_model,
        Some(AgentBackend::Gemini) => &settings.gemini_model,
        Some(AgentBackend::Kimi) => &settings.kimi_model,
        Some(AgentBackend::OpenCode) => &settings.opencode_model,
        None => return String::new(),
    };
    csv.split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
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
    let model = resolve_stage_model(&stage, settings);

    match dispatch_agent(
        backend,
        &model,
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

/// Runs an agent stage that is not tied to an iteration row.
async fn execute_run_level_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    db: &DbPool,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
    let model = resolve_stage_model(&stage, settings);

    match dispatch_agent(
        backend,
        &model,
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

/// Marks a stage as skipped and persists the skip reason.
fn execute_skipped_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    stage: PipelineStage,
    reason: &str,
    db: &DbPool,
) -> StageResult {
    emit_stage(app, run_id, &stage, &StageStatus::Skipped, iteration_num);

    let stage_str = stage_to_str(&stage);
    let _ = db::runs::insert_stage(db, iteration_db_id, &stage_str, "skipped", reason, 0, None);

    StageResult {
        stage,
        status: StageStatus::Skipped,
        output: reason.to_string(),
        duration_ms: 0,
        error: None,
    }
}

/// Runs the plan auditor stage.
async fn execute_plan_audit_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    iteration_db_id: i32,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    db: &DbPool,
) -> StageResult {
    let stage = PipelineStage::PlanAudit;
    let stage_str = stage_to_str(&stage);
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
    let model = resolve_stage_model(&stage, settings);

    match dispatch_agent(
        backend,
        &model,
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

#[derive(Clone, Debug)]
struct PlanAuditParsed {
    verdict: String,
    reasoning: String,
    improved_plan: String,
}

/// Parses plan auditor output.
/// Expected shape:
///   line 1: APPROVED or REJECTED
///   optional reasoning and `--- Improved Plan ---` section
fn parse_plan_audit_output(output: &str, fallback_plan: &str) -> PlanAuditParsed {
    let mut lines = output.lines();
    let first_line = lines.next().unwrap_or("").trim();
    let mut verdict = if first_line == "APPROVED" || first_line == "REJECTED" {
        first_line.to_string()
    } else {
        "INVALID".to_string()
    };

    let remainder = lines.collect::<Vec<_>>().join("\n");
    let marker = "--- Improved Plan ---";
    let alt_marker = "--- Rewritten Plan ---";

    let (reasoning_raw, plan_raw) = if let Some(idx) = remainder.find(marker) {
        let (head, tail) = remainder.split_at(idx);
        (
            head.trim().to_string(),
            tail[marker.len()..].trim().to_string(),
        )
    } else if let Some(idx) = remainder.find(alt_marker) {
        let (head, tail) = remainder.split_at(idx);
        (
            head.trim().to_string(),
            tail[alt_marker.len()..].trim().to_string(),
        )
    } else if verdict == "REJECTED" {
        (remainder.trim().to_string(), String::new())
    } else {
        (String::new(), remainder.trim().to_string())
    };

    let improved = if plan_raw.trim().is_empty() {
        if verdict == "REJECTED" {
            // A rejected plan must still continue; preserve planner output.
            fallback_plan.to_string()
        } else if !remainder.trim().is_empty() {
            remainder.trim().to_string()
        } else {
            fallback_plan.to_string()
        }
    } else {
        plan_raw
    };

    if verdict == "INVALID" && improved.trim().is_empty() {
        verdict = "REJECTED".to_string();
    }

    PlanAuditParsed {
        verdict,
        reasoning: reasoning_raw,
        improved_plan: improved,
    }
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

fn build_planner_prompt(enhanced_prompt: &str, original_prompt: &str) -> String {
    format!(
        "You are the Planner stage in a coding pipeline.\n\
Create a concrete, implementation-ready plan for the coding task.\n\
Be explicit about sequence, edge cases, and verification checks.\n\
Return only the plan content.\n\n\
--- Original Prompt ---\n{original_prompt}\n\n\
--- Enhanced Prompt ---\n{enhanced_prompt}"
    )
}

fn build_plan_auditor_prompt(
    enhanced_prompt: &str,
    original_prompt: &str,
    plan_output: &str,
) -> String {
    format!(
        "You are the Plan Auditor stage in a coding pipeline.\n\
Review the proposed plan against the enhanced prompt.\n\
The first line MUST be exactly APPROVED or REJECTED.\n\
Then improve and rewrite the plan so it is implementation-ready.\n\
Use this exact section header before the rewritten plan: --- Improved Plan ---\n\n\
--- Original Prompt ---\n{original_prompt}\n\n\
--- Enhanced Prompt ---\n{enhanced_prompt}\n\n\
--- Proposed Plan ---\n{plan_output}"
    )
}

fn build_generate_prompt(
    enhanced_prompt: &str,
    original_prompt: &str,
    audited_plan: Option<&str>,
) -> String {
    let plan_section = audited_plan
        .map(|plan| format!("--- Audited Plan ---\n{plan}\n\n"))
        .unwrap_or_default();
    format!(
        "You are a senior software developer implementing the task.\n\
Work directly in the repository and produce complete, correct code changes.\n\n\
--- Original Prompt ---\n{original_prompt}\n\n\
--- Enhanced Prompt ---\n{enhanced_prompt}\n\n\
{plan_section}Return only actionable implementation output."
    )
}

fn build_review_prompt(
    enhanced_prompt: &str,
    original_prompt: &str,
    audited_plan: Option<&str>,
) -> String {
    let plan_section = audited_plan
        .map(|plan| format!("--- Audited Plan ---\n{plan}\n\n"))
        .unwrap_or_default();
    format!(
        "You are a senior software developer performing code review.\n\
--- Original Prompt ---\n{original_prompt}\n\n\
--- Enhanced Prompt ---\n{enhanced_prompt}\n\n\
{plan_section}\
Inspect the repository state yourself using tools (git diff, git status, file reads).\n\
Do not assume a diff is provided in the prompt.\n\
Return specific findings and required fixes."
    )
}

fn build_fix_prompt(
    enhanced_prompt: &str,
    original_prompt: &str,
    audited_plan: Option<&str>,
    review_context: &str,
) -> String {
    let plan_section = audited_plan
        .map(|plan| format!("--- Audited Plan ---\n{plan}\n\n"))
        .unwrap_or_default();
    format!(
        "You are a senior software developer fixing the code.\n\
--- Original Prompt ---\n{original_prompt}\n\n\
--- Enhanced Prompt ---\n{enhanced_prompt}\n\n\
{plan_section}\
--- Review Output ---\n{review_context}\n\n\
Inspect repository changes using tools before editing.\n\
Do not assume a diff is provided in the prompt.\n\
Apply concrete fixes."
    )
}

fn build_judge_prompt(
    enhanced_prompt: &str,
    original_prompt: &str,
    audited_plan: Option<&str>,
    review_output: &str,
    fix_output: &str,
) -> String {
    let plan_section = audited_plan
        .map(|plan| format!("--- Audited Plan ---\n{plan}\n\n"))
        .unwrap_or_default();
    format!(
        "You are the final judge and a senior software developer.\n\
--- Original Prompt ---\n{original_prompt}\n\n\
--- Enhanced Prompt ---\n{enhanced_prompt}\n\n\
{plan_section}\
--- Review Output ---\n{review_output}\n\n\
--- Fix Output ---\n{fix_output}\n\n\
Inspect repository changes using tools (especially git diff) before final judgement.\n\
First line must be COMPLETE or NOT COMPLETE."
    )
}

fn build_executive_summary_prompt() -> String {
    "You are the Executive Summary stage.\n\
Summarise the full pipeline run for future agents.\n\
Keep it concise and factual: objective, decisions, what changed, unresolved risks, and next steps.\n\
Output plain text only."
        .to_string()
}

fn build_executive_summary_context(run: &PipelineRun) -> String {
    let mut lines = vec![
        format!("Run ID: {}", run.id),
        format!("Prompt: {}", run.prompt),
        format!("Status: {:?}", run.status),
        format!("Max Iterations: {}", run.max_iterations),
        format!("Completed Iterations: {}", run.current_iteration),
    ];
    if let Some(verdict) = run.final_verdict.as_ref() {
        lines.push(format!("Final Verdict: {:?}", verdict));
    }
    if let Some(error) = run.error.as_ref() {
        lines.push(format!("Error: {error}"));
    }
    for iteration in &run.iterations {
        lines.push(format!("Iteration {}:", iteration.number));
        for stage in &iteration.stages {
            lines.push(format!(
                "- {:?}: {:?} ({}ms)",
                stage.stage, stage.status, stage.duration_ms
            ));
        }
        if let Some(reasoning) = iteration.judge_reasoning.as_ref() {
            lines.push(format!("Judge Reasoning: {reasoning}"));
        }
    }
    lines.join("\n")
}

fn backend_to_db_str(backend: &AgentBackend) -> &'static str {
    match backend {
        AgentBackend::Claude => "claude",
        AgentBackend::Codex => "codex",
        AgentBackend::Gemini => "gemini",
        AgentBackend::Kimi => "kimi",
        AgentBackend::OpenCode => "opencode",
    }
}

fn add_plan_context(
    original_prompt: &str,
    base_context: Option<String>,
    audited_plan: Option<&str>,
) -> String {
    let mut sections = vec![format!("--- Original Prompt ---\n{original_prompt}")];
    if let Some(plan) = audited_plan {
        sections.push(format!("--- Audited Plan ---\n{plan}"));
    }
    if let Some(context) = base_context {
        if !context.trim().is_empty() {
            sections.push(context);
        }
    }
    sections.join("\n\n")
}

/// Runs the full orchestration pipeline:
///   prompt enhance → plan → plan audit → generate → diff → review → [ask user] → fix → diff → judge → loop
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
        let mut iter_ctx = IterationContext::new(request.prompt.clone());

        // --- 1. Prompt enhance ---
        let prompt_enhance_input = AgentInput {
            prompt: build_prompt_enhancer_prompt(&request.prompt),
            context: None,
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
        iter_ctx.enhanced_prompt = enhanced_prompt.clone();
        persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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

        let planning_enabled =
            settings.planner_agent.is_some() && settings.plan_auditor_agent.is_some();

        if planning_enabled {
            // --- 2. Plan ---
            let plan_input = AgentInput {
                prompt: build_planner_prompt(&enhanced_prompt),
                context: Some(stage_context_with_original(&request.prompt, None)),
                workspace_path: request.workspace_path.clone(),
            };
            run.current_stage = Some(PipelineStage::Plan);
            let plan_result = execute_agent_stage(
                &app,
                &run_id,
                iter_num,
                iteration_db_id,
                PipelineStage::Plan,
                settings
                    .planner_agent
                    .as_ref()
                    .expect("planner_agent is guaranteed when planning_enabled"),
                &plan_input,
                &settings,
                Some(session_id.as_str()),
                &db,
            )
            .await;
            let plan_output = plan_result.output.clone();
            iter_ctx.planner_plan = Some(plan_output.clone());
            persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
            let plan_failed = plan_result.status == StageStatus::Failed;
            emit_artifact(&app, &run_id, "plan", &plan_output, iter_num, &db);
            stages.push(plan_result);
            if plan_failed || is_cancelled(&cancel_flag) {
                run.iterations.push(Iteration {
                    number: iter_num,
                    stages,
                    verdict: None,
                    judge_reasoning: None,
                });
                if plan_failed {
                    run.status = PipelineStatus::Failed;
                    run.error = Some("Planner stage failed".to_string());
                }
                break;
            }

            // --- 3. Plan audit ---
            let plan_audit_input = AgentInput {
                prompt: build_plan_auditor_prompt(&enhanced_prompt, &plan_output),
                context: Some(stage_context_with_original(&request.prompt, None)),
                workspace_path: request.workspace_path.clone(),
            };
            run.current_stage = Some(PipelineStage::PlanAudit);
            let plan_audit_result = execute_plan_audit_stage(
                &app,
                &run_id,
                iter_num,
                iteration_db_id,
                settings
                    .plan_auditor_agent
                    .as_ref()
                    .expect("plan_auditor_agent is guaranteed when planning_enabled"),
                &plan_audit_input,
                &settings,
                Some(session_id.as_str()),
                &db,
            )
            .await;
            let plan_audit_output = plan_audit_result.output.clone();
            let plan_audit_failed = plan_audit_result.status == StageStatus::Failed;
            emit_artifact(
                &app,
                &run_id,
                "plan_audit",
                &plan_audit_output,
                iter_num,
                &db,
            );
            stages.push(plan_audit_result.clone());
            if plan_audit_failed || is_cancelled(&cancel_flag) {
                run.iterations.push(Iteration {
                    number: iter_num,
                    stages,
                    verdict: None,
                    judge_reasoning: None,
                });
                if plan_audit_failed {
                    run.status = PipelineStatus::Failed;
                    run.error = Some(
                        plan_audit_result
                            .error
                            .clone()
                            .unwrap_or_else(|| "Plan Auditor stage failed".to_string()),
                    );
                }
                break;
            }

            let parsed = parse_plan_audit_output(&plan_audit_output, &plan_output);
            iter_ctx.audit_verdict = Some(parsed.verdict);
            iter_ctx.audit_reasoning = if parsed.reasoning.trim().is_empty() {
                None
            } else {
                Some(parsed.reasoning)
            };
            iter_ctx.audited_plan = Some(parsed.improved_plan);
            persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
            if let Some(plan) = iter_ctx.audited_plan.as_ref() {
                emit_artifact(&app, &run_id, "plan_final", plan, iter_num, &db);
            }
        } else {
            // --- 2. Plan (skipped) / 3. Plan audit (skipped) ---
            let skip_reason =
                "Planner and Plan Auditor must both be selected; skipping planning stages.";
            run.current_stage = Some(PipelineStage::Plan);
            let skipped_plan = execute_skipped_stage(
                &app,
                &run_id,
                iter_num,
                iteration_db_id,
                PipelineStage::Plan,
                skip_reason,
                &db,
            );
            stages.push(skipped_plan);

            run.current_stage = Some(PipelineStage::PlanAudit);
            let skipped_audit = execute_skipped_stage(
                &app,
                &run_id,
                iter_num,
                iteration_db_id,
                PipelineStage::PlanAudit,
                skip_reason,
                &db,
            );
            stages.push(skipped_audit);
        }

        // --- 4. Generate ---
        let gen_input = AgentInput {
            prompt: enhanced_prompt.clone(),
            context: Some(add_plan_context(
                &request.prompt,
                None,
                iter_ctx.selected_plan(),
            )),
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
            iter_ctx.generate_question = Some(question.clone());
            persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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
                    iter_ctx.generate_answer = Some(a.answer.clone());
                    persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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

        // --- 5. Diff after generate ---
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
        stages.push(diff1);
        if is_cancelled(&cancel_flag) {
            push_cancel_iteration(&mut run, iter_num, stages);
            break;
        }

        // --- 6. Review ---
        let review_input = AgentInput {
            prompt: build_review_prompt(&enhanced_prompt),
            context: Some(add_plan_context(
                &request.prompt,
                None,
                iter_ctx.selected_plan(),
            )),
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
        iter_ctx.review_output = Some(review_output.clone());
        persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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
                iter_ctx.review_user_guidance = Some(answer.answer.clone());
                persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
                format!(
                    "{}\n\n--- User Guidance ---\n{}",
                    review_output, answer.answer
                )
            }
            _ => review_output.clone(),
        };

        // --- 7. Fix ---
        let fix_input = AgentInput {
            prompt: build_fix_prompt(&enhanced_prompt),
            context: Some(add_plan_context(
                &request.prompt,
                Some(fix_context),
                iter_ctx.selected_plan(),
            )),
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
        iter_ctx.fix_output = Some(fix_output.clone());
        persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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
            iter_ctx.fix_question = Some(question.clone());
            persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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
            if let Some(a) = answer {
                if !a.skipped && !a.answer.is_empty() {
                    iter_ctx.fix_answer = Some(a.answer);
                    persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
                }
            }
        }

        // --- 8. Diff after fix ---
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
        stages.push(diff2);
        if is_cancelled(&cancel_flag) {
            push_cancel_iteration(&mut run, iter_num, stages);
            break;
        }

        // --- 9. Judge ---
        let judge_context = format!(
            "--- Review ---\n{}\n\n--- Fix ---\n{}",
            review_output, fix_output
        );
        let judge_input = AgentInput {
            prompt: build_judge_prompt(&enhanced_prompt),
            context: Some(add_plan_context(
                &request.prompt,
                Some(judge_context),
                iter_ctx.selected_plan(),
            )),
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
        iter_ctx.judge_output = Some(judge_output.clone());
        persist_iteration_context(&db, &run_id, iter_num, &iter_ctx);
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

    // --- Executive summary (run-level) ---
    let summary_iteration = run.current_iteration;
    run.current_stage = Some(PipelineStage::ExecutiveSummary);
    let summary_input = AgentInput {
        prompt: build_executive_summary_prompt(),
        context: Some(build_executive_summary_context(&run)),
        workspace_path: request.workspace_path.clone(),
    };
    let summary_result = execute_run_level_agent_stage(
        &app,
        &run_id,
        summary_iteration,
        PipelineStage::ExecutiveSummary,
        &settings.executive_summary_agent,
        &summary_input,
        &settings,
        Some(session_id.as_str()),
        &db,
    )
    .await;
    let summary_model = resolve_stage_model(&PipelineStage::ExecutiveSummary, &settings);
    let summary_generated_at = chrono::Utc::now().to_rfc3339();
    let summary_status = if summary_result.status == StageStatus::Completed {
        "completed"
    } else {
        "failed"
    };
    let patch = db::runs::RunExecutiveSummaryPatch {
        executive_summary: if summary_result.status == StageStatus::Completed {
            Some(summary_result.output.as_str())
        } else {
            None
        },
        executive_summary_status: Some(summary_status),
        executive_summary_error: summary_result.error.as_deref(),
        executive_summary_agent: Some(backend_to_db_str(&settings.executive_summary_agent)),
        executive_summary_model: Some(summary_model.as_str()),
        executive_summary_generated_at: Some(summary_generated_at.as_str()),
    };
    let _ = db::runs::update_executive_summary(&db, &run_id, &patch);
    if summary_result.status == StageStatus::Completed {
        emit_artifact(
            &app,
            &run_id,
            "executive_summary",
            &summary_result.output,
            summary_iteration,
            &db,
        );
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

#[cfg(test)]
mod tests {
    use super::parse_plan_audit_output;

    #[test]
    fn parse_plan_audit_approved_with_improved_plan() {
        let raw = "APPROVED\nLooks good.\n--- Improved Plan ---\n1. Do A\n2. Do B";
        let parsed = parse_plan_audit_output(raw, "fallback");
        assert_eq!(parsed.verdict, "APPROVED");
        assert_eq!(parsed.improved_plan, "1. Do A\n2. Do B");
    }

    #[test]
    fn parse_plan_audit_rejected_with_rewrite_continues() {
        let raw = "REJECTED\nMissing checks.\n--- Improved Plan ---\n1. Add checks";
        let parsed = parse_plan_audit_output(raw, "fallback");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "1. Add checks");
    }

    #[test]
    fn parse_plan_audit_rejected_without_rewrite_uses_fallback() {
        let raw = "REJECTED\nNo rewrite provided.";
        let parsed = parse_plan_audit_output(raw, "fallback plan");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "fallback plan");
    }
}
