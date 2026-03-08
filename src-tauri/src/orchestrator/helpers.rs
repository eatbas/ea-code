//! Utility functions for the orchestration pipeline:
//! agent dispatch, event emission, cancellation, and context persistence.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::agents::{
    run_claude, run_codex, run_gemini, run_kimi, run_opencode, AgentInput, AgentOutput,
};
use crate::db::{self, DbPool};
use crate::events::*;
use crate::models::*;

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time as epoch milliseconds (string).
pub fn epoch_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

/// Serialises a PipelineStage to its snake_case string for DB storage.
pub fn stage_to_str(stage: &PipelineStage) -> String {
    serde_json::to_value(stage)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{stage:?}"))
}

/// Dispatches to the appropriate agent runner based on the backend setting.
pub async fn dispatch_agent(
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
                input, &settings.claude_path, model, session_id,
                app, run_id, stage, db,
            )
            .await
        }
        AgentBackend::Codex => {
            run_codex(input, &settings.codex_path, model, app, run_id, stage, db).await
        }
        AgentBackend::Gemini => {
            run_gemini(input, &settings.gemini_path, model, app, run_id, stage, db).await
        }
        AgentBackend::Kimi => {
            run_kimi(input, &settings.kimi_path, model, app, run_id, stage, db).await
        }
        AgentBackend::OpenCode => {
            run_opencode(input, &settings.opencode_path, model, app, run_id, stage, db).await
        }
    }
}

/// Resolves the model to use for a given pipeline stage from per-stage settings.
pub fn resolve_stage_model(stage: &PipelineStage, settings: &AppSettings) -> String {
    match stage {
        PipelineStage::PromptEnhance => settings.prompt_enhancer_model.clone(),
        PipelineStage::SkillSelect => settings
            .skill_selector_model
            .clone()
            .unwrap_or_else(|| first_enabled_model_for_backend(settings.skill_selector_agent.as_ref(), settings)),
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
        PipelineStage::DiffAfterGenerate | PipelineStage::DiffAfterFix => String::new(),
    }
}

fn first_enabled_model_for_backend(
    backend: Option<&AgentBackend>,
    settings: &AppSettings,
) -> String {
    let csv = match backend {
        Some(AgentBackend::Claude) => &settings.claude_model,
        Some(AgentBackend::Codex) => &settings.codex_model,
        Some(AgentBackend::Gemini) => &settings.gemini_model,
        Some(AgentBackend::Kimi) => &settings.kimi_model,
        Some(AgentBackend::OpenCode) => &settings.opencode_model,
        None => return String::new(),
    };
    csv.split(',').next().unwrap_or("").trim().to_string()
}

/// Emits a stage status transition event.
pub fn emit_stage(
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
pub fn emit_artifact(
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

pub fn is_cancelled(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

pub async fn wait_for_cancel(cancel_flag: &Arc<AtomicBool>) {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

pub fn push_cancel_iteration(run: &mut PipelineRun, iter_num: u32, stages: Vec<StageResult>) {
    run.iterations.push(Iteration {
        number: iter_num,
        stages,
        verdict: None,
        judge_reasoning: None,
    });
    run.status = PipelineStatus::Cancelled;
}

pub fn backend_to_db_str(backend: &AgentBackend) -> &'static str {
    match backend {
        AgentBackend::Claude => "claude",
        AgentBackend::Codex => "codex",
        AgentBackend::Gemini => "gemini",
        AgentBackend::Kimi => "kimi",
        AgentBackend::OpenCode => "opencode",
    }
}

/// Persists iteration context to the database.
pub fn persist_iteration_context(
    db: &DbPool,
    run_id: &str,
    iteration: u32,
    context: &super::pipeline::IterationContext,
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
