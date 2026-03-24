//! Utility functions for the orchestration pipeline:
//! agent dispatch, event emission, cancellation, and context persistence.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use tauri::Manager;

use crate::agents::api_client;
use crate::agents::{AgentInput, AgentOutput};
use crate::commands::AppState;
use crate::events::*;
use crate::models::*;
use crate::storage::runs;

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time as epoch milliseconds (string).
pub fn epoch_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

/// Serialises a PipelineStage to its snake_case string.
#[allow(dead_code)]
pub fn stage_to_str(stage: &PipelineStage) -> String {
    serde_json::to_value(stage)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{stage:?}"))
}

/// Detects output that is a file-write instruction rather than real content.
///
/// Some CLIs (notably Claude Code) may instruct the model to write plans to
/// `PLAN.md` rather than returning the text inline. When that happens the
/// captured stdout contains only a short directive like:
///   `--- OUTPUT FILE ---\nWrite your plan to this file: …\PLAN.md`
pub(crate) fn looks_like_output_file_instruction(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("--- output file ---") || lower.contains("write your plan to this file"))
        && lower.contains("plan.md")
}

/// Result of dispatching an agent, including session continuity metadata.
pub struct DispatchResult {
    pub output: AgentOutput,
    pub provider_session_ref: Option<String>,
}

/// Dispatches to the appropriate agent runner based on the backend setting.
///
/// When `output_file` is provided, the agent's prompt is augmented with an
/// instruction to write its output to that file. After the agent finishes,
/// the file is read and its content becomes the returned `AgentOutput`.
///
/// When `session_ref` is provided, the agent continues an existing CLI
/// session (mode "resume") instead of starting a fresh one.
pub async fn dispatch_agent(
    backend: &AgentBackend,
    model: &str,
    input: &AgentInput,
    _settings: &AppSettings,
    _session_id: Option<&str>,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    output_file: Option<&str>,
    session_ref: Option<&str>,
) -> Result<DispatchResult, String> {
    if model.is_empty() {
        eprintln!(
            "[warn] No model configured for stage {:?} with backend {:?}; the CLI will use its default.",
            stage, backend,
        );
    }

    let intent = stage.execution_intent();
    if stage.requires_output_file() && output_file.is_none() {
        return Err(format!(
            "{stage:?} is a text stage and requires an output artifact path"
        ));
    }

    // For text-only stages, derive a workspace-local temp file path so CLIs
    // with native file-output support can write there safely. We no longer ask
    // the model itself to create the file.
    let workspace_output_file = output_file.map(|artifact_path| {
        let stem = std::path::Path::new(artifact_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let name = format!(".ea-pipeline-{stem}.md");
        let p = std::path::Path::new(&input.workspace_path).join(&name);
        // Remove any stale file from a previous run.
        let _ = std::fs::remove_file(&p);
        (p, name)
    });
    let _workspace_output_file_str = workspace_output_file
        .as_ref()
        .map(|(path, _)| path.to_string_lossy().to_string());

    // Ensure the hive-api sidecar is running before dispatching.
    let app_state = app.state::<AppState>();
    app_state.sidecar.ensure_running().await?;
    let base_url = app_state.sidecar.base_url().await;

    let provider = backend_to_provider_str(backend);

    let api_result = api_client::run_api_agent(
        &base_url,
        input,
        provider,
        model,
        app,
        run_id,
        stage.clone(),
        session_ref,
    )
    .await;

    // Track or clear the active job_id for cancellation support.
    match &api_result {
        Ok(r) if !r.job_id.is_empty() => {
            app_state
                .active_jobs
                .lock()
                .await
                .insert(run_id.to_string(), r.job_id.clone());
        }
        _ => {
            app_state.active_jobs.lock().await.remove(run_id);
        }
    }

    let api_ok = api_result?;
    let result = api_ok.output;
    let captured_session_ref = api_ok.provider_session_ref;

    // For text stages, persist the final textual artifact even when the CLI
    // only emitted to stdout. Native file-output is preferred when available.
    if matches!(intent, StageExecutionIntent::Text) {
        let mut final_text = result.raw_text.trim().to_string();
        if let Some((ref ws_file, _)) = workspace_output_file {
            if let Ok(content) = std::fs::read_to_string(ws_file) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    final_text = trimmed.to_string();
                }
            }
            let _ = std::fs::remove_file(ws_file);
        }

        // Fallback: some CLIs (e.g. Claude Code) may write to PLAN.md in the
        // workspace instead of returning the plan inline. If the captured output
        // looks like a file-write instruction rather than real content, try
        // reading PLAN.md from the workspace.
        if looks_like_output_file_instruction(&final_text) || final_text.is_empty() {
            let plan_md = std::path::Path::new(&input.workspace_path).join("PLAN.md");
            if let Ok(content) = std::fs::read_to_string(&plan_md) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    final_text = trimmed.to_string();
                    let _ = std::fs::remove_file(&plan_md);
                }
            }
        }

        if let Some(artifact_path) = output_file {
            let _ = std::fs::write(artifact_path, &final_text);
        }
        return Ok(DispatchResult {
            output: AgentOutput {
                raw_text: final_text,
            },
            provider_session_ref: captured_session_ref,
        });
    }

    Ok(DispatchResult {
        output: result,
        provider_session_ref: captured_session_ref,
    })
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
        PipelineStage::ExtraPlan(i) => {
            let slot = settings.extra_planners.get(*i as usize);
            resolve_model_with_fallback(
                slot.and_then(|s| s.model.as_deref()),
                slot.and_then(|s| s.agent.as_ref()),
                settings,
            )
        }
        PipelineStage::ExtraReviewer(i) => {
            let slot = settings.extra_reviewers.get(*i as usize);
            resolve_model_with_fallback(
                slot.and_then(|s| s.model.as_deref()),
                slot.and_then(|s| s.agent.as_ref()),
                settings,
            )
        }
        PipelineStage::ReviewMerge => resolve_model_with_fallback(
            settings.review_merger_model.as_deref(),
            settings.review_merger_agent.as_ref(),
            settings,
        ),
        PipelineStage::DirectTask => String::new(),
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
    let backend = match backend {
        Some(b) => b,
        None => return String::new(),
    };
    let provider_str = backend_to_provider_str(backend);
    // Check dynamic provider_models first, then fall back to legacy fields.
    let csv = settings
        .provider_models
        .get(provider_str)
        .map(|s| s.as_str())
        .or_else(|| settings.model_csv_for_cli(provider_str))
        .unwrap_or("");
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
    emit_stage_with_duration(app, run_id, stage, status, iteration, None);
}

/// Emits a stage status transition event with an optional duration.
pub fn emit_stage_with_duration(
    app: &AppHandle,
    run_id: &str,
    stage: &PipelineStage,
    status: &StageStatus,
    iteration: u32,
    duration_ms: Option<u64>,
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

    // Update summary.json with current stage for crash recovery
    if let Ok(mut summary) = runs::read_summary(run_id) {
        if matches!(status, StageStatus::Running) {
            summary.current_stage = Some(stage.clone());
            let _ = runs::update_summary(run_id, &summary);
        }
    }
}

/// Emits a pipeline artefact event so the frontend can display stage outputs,
/// and persists the artefact to disk for historical viewing.
pub fn emit_artifact(app: &AppHandle, run_id: &str, kind: &str, content: &str, iteration: u32) {
    let _ = app.emit(
        EVENT_PIPELINE_ARTIFACT,
        PipelineArtifactPayload {
            run_id: run_id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            iteration,
        },
    );

    // Persist artefact to disk for historical viewing
    if let Err(e) = runs::write_artifact(run_id, iteration, kind, content) {
        eprintln!("Warning: Failed to persist artefact '{kind}': {e}");
    }
}

/// Persists the full prompt sent to an agent as a `{kind}_input.md` artifact.
///
/// This captures the complete system + user prompt for debugging and
/// reproducibility. Does not emit a frontend event — input prompts are
/// only persisted to disk.
pub fn emit_prompt_artifact(run_id: &str, kind: &str, input: &AgentInput, iteration: u32) {
    let mut content = input.prompt.clone();
    if let Some(ref ctx) = input.context {
        content = format!("{ctx}\n\n---\n\n{content}");
    }
    let input_kind = format!("{kind}_input");
    if let Err(e) = runs::write_artifact(run_id, iteration, &input_kind, &content) {
        eprintln!("Warning: Failed to persist prompt artifact '{input_kind}': {e}");
    }
}

pub fn is_cancelled(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunInterrupt {
    Pause,
    Cancel,
}

pub async fn wait_for_interrupt(
    pause_flag: &Arc<AtomicBool>,
    cancel_flag: &Arc<AtomicBool>,
) -> RunInterrupt {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return RunInterrupt::Cancel;
        }
        if pause_flag.load(Ordering::SeqCst) {
            return RunInterrupt::Pause;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

pub async fn wait_if_paused(pause_flag: &Arc<AtomicBool>, cancel_flag: &Arc<AtomicBool>) -> bool {
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

/// Returns the session pair name for a pipeline stage.
///
/// Stages within the same pair share a CLI session for context continuity.
/// Currently: Plan+PlanAudit+Review+ReviewMerge share "plan_review",
/// Coder+CodeFixer share "code_fix", others get their own.
pub fn session_pair_for_stage(stage: &PipelineStage) -> &'static str {
    match stage {
        PipelineStage::Plan
        | PipelineStage::ExtraPlan(_)
        | PipelineStage::PlanAudit
        | PipelineStage::CodeReviewer
        | PipelineStage::ExtraReviewer(_)
        | PipelineStage::ReviewMerge => "plan_review",

        PipelineStage::Coder | PipelineStage::CodeFixer => "code_fix",
        PipelineStage::PromptEnhance => "enhance",
        PipelineStage::Judge => "judge",
        PipelineStage::SkillSelect => "skill_select",
        PipelineStage::ExecutiveSummary => "executive_summary",
        PipelineStage::DirectTask => "direct_task",
    }
}

/// Maps an AgentBackend to the hive-api provider string.
pub fn backend_to_provider_str(backend: &AgentBackend) -> &'static str {
    match backend {
        AgentBackend::Claude => "claude",
        AgentBackend::Codex => "codex",
        AgentBackend::Gemini => "gemini",
        AgentBackend::Kimi => "kimi",
        AgentBackend::OpenCode => "opencode",
        AgentBackend::Copilot => "copilot",
    }
}

#[allow(dead_code)]
pub fn backend_to_db_str(backend: &AgentBackend) -> &'static str {
    backend_to_provider_str(backend)
}

/// Tracks CLI session references across pipeline stages within a run.
///
/// Session refs are looked up before dispatch and stored after.
/// The tracker persists to `cli_sessions.json` after each update.
pub struct CliSessionTracker {
    run_id: String,
    sessions: HashMap<String, String>, // pair_name -> provider_session_ref
}

impl CliSessionTracker {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            sessions: HashMap::new(),
        }
    }

    /// Gets the session ref for a stage's pair, if one exists.
    #[allow(dead_code)]
    pub fn get_ref_for_stage(&self, stage: &PipelineStage) -> Option<&str> {
        let pair = session_pair_for_stage(stage);
        self.sessions.get(pair).map(|s| s.as_str())
    }

    /// Stores a session ref returned from a stage and persists to disk.
    pub fn store_ref_from_result(&mut self, result: &StageResult) {
        if let Some(ref session_ref) = result.provider_session_ref {
            let pair = session_pair_for_stage(&result.stage);
            self.sessions.insert(pair.to_string(), session_ref.clone());

            // Persist to disk (best-effort)
            let entry = CliSessionEntry {
                session_ref: session_ref.clone(),
                backend: AgentBackend::Claude, // Will be refined when backend is passed
                model: String::new(),
                stages_used: vec![result.stage.clone()],
                created_at: crate::storage::now_rfc3339(),
                last_used_at: crate::storage::now_rfc3339(),
            };
            if let Err(e) = crate::storage::runs::update_cli_session(&self.run_id, pair, entry) {
                eprintln!("Warning: Failed to persist CLI session ref for {pair}: {e}");
            }
        }
    }
}
