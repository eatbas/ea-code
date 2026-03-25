//! Agent dispatch routing and artifact matching helpers.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::agents::api_client;
use crate::agents::{AgentInput, AgentOutput};
use crate::commands::AppState;
use crate::models::*;

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
    abort_flag: Option<Arc<AtomicBool>>,
) -> Result<DispatchResult, String> {
    if model.is_empty() {
        eprintln!(
            "[warn] No model configured for stage {:?} with backend {:?}; the CLI will use its default.",
            stage, backend,
        );
    }

    let intent = stage.execution_intent();

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
        abort_flag,
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

    // For text stages, try to read content from the output file or any
    // matching file the agent may have created in the same directory.
    // Prefer file content over stdout since the file is the authoritative
    // artifact (the agent may have refined it further after emitting to stdout).
    if matches!(intent, StageExecutionIntent::Text) {
        let file_content = output_file
            .and_then(|expected| {
                // First try the exact path.
                if let Ok(content) = std::fs::read_to_string(expected) {
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() {
                        return Some(trimmed);
                    }
                }
                // Scan the directory for any .md file the agent may have
                // written under a different name.
                find_matching_artifact(expected)
            });

        let final_text = file_content.unwrap_or_else(|| result.raw_text.trim().to_string());

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

/// Extracts the stage prefix from a descriptive artifact filename.
/// `plan_1_claude_opus-4.md` -> `plan`
/// `review_2_copilot_gpt-5.4-mini.md` -> `review`
fn artifact_prefix(filename: &str) -> &str {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    // Find the first '_' followed by a digit -- everything before is the prefix.
    if let Some(pos) = stem.find(|c: char| c == '_') {
        let after = &stem[pos + 1..];
        if after.starts_with(|c: char| c.is_ascii_digit()) {
            return &stem[..pos];
        }
    }
    stem
}

/// Scans the iteration directory (parent of `expected_path`) for any `.md` file
/// whose name starts with the same stage prefix (e.g. "plan", "review").
/// Returns the content of the first match found.
fn find_matching_artifact(expected_path: &str) -> Option<String> {
    use std::path::Path;

    let expected = Path::new(expected_path);
    let dir = expected.parent()?;
    let expected_name = expected.file_name()?.to_string_lossy();
    let prefix = artifact_prefix(&expected_name);

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.ends_with(".md") {
            continue;
        }
        // Skip files that don't start with the same prefix.
        if !name_str.starts_with(prefix) {
            continue;
        }
        // Skip the exact file we already tried.
        if name_str == *expected_name {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                eprintln!(
                    "[info] Found matching artifact '{}' (expected '{}')",
                    name_str, expected_name,
                );
                return Some(trimmed);
            }
        }
    }
    None
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

/// Returns the session pair key for a pipeline stage.
///
/// Stages within the same pair share a CLI session for context continuity.
/// Per-slot pairing: Planner[0] and Reviewer[0] share "plan_review_0",
/// Planner[1] and Reviewer[1] share "plan_review_1", etc.
/// Coder and CodeFixer share "code_fix".
pub fn session_pair_for_stage(stage: &PipelineStage) -> String {
    match stage {
        // Per-slot pairing: planner N pairs with reviewer N.
        PipelineStage::Plan => "plan_review_0".to_string(),
        PipelineStage::ExtraPlan(i) => format!("plan_review_{}", i + 1),
        PipelineStage::CodeReviewer => "plan_review_0".to_string(),
        PipelineStage::ExtraReviewer(i) => format!("plan_review_{}", i + 1),

        // Aggregator stages get their own sessions.
        PipelineStage::PlanAudit => "plan_audit".to_string(),
        PipelineStage::ReviewMerge => "review_merge".to_string(),

        PipelineStage::Coder | PipelineStage::CodeFixer => "code_fix".to_string(),
        PipelineStage::PromptEnhance => "enhance".to_string(),
        PipelineStage::Judge => "judge".to_string(),
        PipelineStage::SkillSelect => "skill_select".to_string(),
        PipelineStage::ExecutiveSummary => "executive_summary".to_string(),
        PipelineStage::DirectTask => "direct_task".to_string(),
    }
}
