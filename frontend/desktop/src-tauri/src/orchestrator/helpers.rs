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
    if model.is_empty() {
        eprintln!(
            "[warn] No model configured for stage {:?} with backend {:?}; the CLI will use its default.",
            stage, backend,
        );
    }
    match backend {
        AgentBackend::Claude => {
            run_claude(
                input, &settings.claude_path, model, settings.agent_max_turns, session_id,
                app, run_id, stage, db,
            )
            .await
        }
        AgentBackend::Codex => {
            run_codex(input, &settings.codex_path, model, session_id, app, run_id, stage, db).await
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
        PipelineStage::PromptEnhance => resolve_model_with_fallback(
            Some(settings.prompt_enhancer_model.as_str()),
            settings.prompt_enhancer_agent.as_ref(),
            settings,
        ),
        PipelineStage::SkillSelect => resolve_model_with_fallback(
            settings.skill_selector_model.as_deref(),
            settings.skill_selector_agent.as_ref(),
            settings,
        ),
        PipelineStage::Plan => resolve_model_with_fallback(
            settings.planner_model.as_deref(),
            settings.planner_agent.as_ref(),
            settings,
        ),
        PipelineStage::PlanAudit => resolve_model_with_fallback(
            settings.plan_auditor_model.as_deref(),
            settings.plan_auditor_agent.as_ref(),
            settings,
        ),
        PipelineStage::Coder => resolve_model_with_fallback(
            Some(settings.coder_model.as_str()),
            settings.coder_agent.as_ref(),
            settings,
        ),
        PipelineStage::CodeReviewer => resolve_model_with_fallback(
            Some(settings.code_reviewer_model.as_str()),
            settings.code_reviewer_agent.as_ref(),
            settings,
        ),
        PipelineStage::CodeFixer => resolve_model_with_fallback(
            Some(settings.code_fixer_model.as_str()),
            settings.code_fixer_agent.as_ref(),
            settings,
        ),
        PipelineStage::Judge => resolve_model_with_fallback(
            Some(settings.final_judge_model.as_str()),
            settings.final_judge_agent.as_ref(),
            settings,
        ),
        PipelineStage::ExecutiveSummary => resolve_model_with_fallback(
            Some(settings.executive_summary_model.as_str()),
            settings.executive_summary_agent.as_ref(),
            settings,
        ),
        PipelineStage::DiffAfterCoder | PipelineStage::DiffAfterCodeFixer | PipelineStage::DirectTask => String::new(),
    }
}

fn resolve_model_with_fallback(
    explicit: Option<&str>,
    backend: Option<&AgentBackend>,
    settings: &AppSettings,
) -> String {
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let enabled = first_enabled_model_for_backend(backend, settings);
    if !enabled.is_empty() {
        return enabled;
    }

    String::new()
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

/// Emits a stage status transition event and persists current stage to DB.
pub fn emit_stage(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
    db: &DbPool,
) {
    emit_stage_with_duration(app, run_id, stage, status, iteration, None, db);
}

/// Emits a stage status transition event with an optional duration, and
/// persists the current stage to the DB so polled views can show progress.
pub fn emit_stage_with_duration(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
    duration_ms: Option<u64>,
    db: &DbPool,
) {
    let _ = app.emit(
        EVENT_PIPELINE_STAGE,
        PipelineStagePayload {
            run_id: run_id.to_string(),
            stage: stage.clone(),
            status: status.clone(),
            iteration,
            duration_ms,
        },
    );

    // Persist current stage to DB for polled session views
    if matches!(status, StageStatus::Running) {
        let _ = db::runs::update_current_stage(
            db,
            run_id,
            Some(&stage_to_str(stage)),
            iteration as i32,
        );
    }
}

/// Transient artefact kinds that are either rebuilt every run or already stored
/// canonically in `stages.output`. These are emitted via Tauri events for
/// real-time display but skipped when writing to the `artifacts` table.
const TRANSIENT_ARTIFACT_KINDS: &[&str] = &[
    // Rebuilt fresh every run — zero historical value
    "workspace_context",
    "session_memory",
    // Already stored in stages.output (canonical) — no need to duplicate
    "result",
    "plan",
    "plan_audit",
    "review",
    "judge",
    "diff",
];

/// Emits an artefact event and persists it to the database.
/// Transient kinds (workspace_context, session_memory) are emitted for real-time
/// display but not written to the database.
pub fn emit_artifact(
    app: &AppHandle,
    run_id: &str,
    kind: &str,
    content: &str,
    iteration: u32,
    db: &DbPool,
) {
    let _ = app.emit(
        EVENT_PIPELINE_ARTIFACT,
        PipelineArtifactPayload {
            run_id: run_id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            iteration,
        },
    );
    if !TRANSIENT_ARTIFACT_KINDS.contains(&kind) {
        let _ = db::artifacts::insert(db, run_id, iteration as i32, kind, content);
    }
}

pub fn is_cancelled(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

pub async fn wait_if_paused(
    pause_flag: &Arc<AtomicBool>,
    cancel_flag: &Arc<AtomicBool>,
) -> bool {
    while pause_flag.load(Ordering::SeqCst) {
        if cancel_flag.load(Ordering::SeqCst) {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    false
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

#[allow(dead_code)]
pub fn backend_to_db_str(backend: &AgentBackend) -> &'static str {
    match backend {
        AgentBackend::Claude => "claude",
        AgentBackend::Codex => "codex",
        AgentBackend::Gemini => "gemini",
        AgentBackend::Kimi => "kimi",
        AgentBackend::OpenCode => "opencode",
    }
}

