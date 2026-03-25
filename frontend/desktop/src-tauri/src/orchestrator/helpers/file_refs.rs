//! File reference helpers and model resolution for pipeline stages.

use super::dispatch::backend_to_provider_str;
use crate::models::*;
use crate::storage::runs;

/// Builds a file-reference instruction for embedding in agent prompts.
///
/// The agent reads the file itself instead of receiving the content inline.
/// This keeps prompts small even when referenced content is large.
pub fn file_ref(path: &std::path::Path) -> String {
    format!("Read the full content from this file: {}", path.display())
}

/// Returns the artifact path for a completed stage, or `None` if the path
/// cannot be resolved (e.g. missing run/session index entry).
pub fn artifact_file_path(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    iteration: u32,
    kind: &str,
) -> Option<std::path::PathBuf> {
    runs::artifact_output_path(workspace_path, session_id, run_id, iteration, kind).ok()
}

/// Returns a file-reference instruction for an artifact when the path can be
/// resolved, otherwise falls back to `fallback` (typically the inline content).
///
/// This is the common 3-step pattern: resolve path -> wrap in `file_ref` -> or
/// fall back to inline text.
pub fn file_ref_or_inline(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    iteration: u32,
    kind: &str,
    fallback: &str,
) -> String {
    artifact_file_path(workspace_path, session_id, run_id, iteration, kind)
        .map(|p| file_ref(&p))
        .unwrap_or_else(|| fallback.to_string())
}

/// Builds a descriptive artifact name that includes the slot index, backend,
/// and model -- e.g. `plan_1_claude_opus-4` or `review_2_copilot_gpt-5.4-mini`.
/// This prevents parallel agents from colliding on the same filename and makes
/// it easy to identify which agent produced which artifact.
pub fn descriptive_artifact_name(prefix: &str, index: usize, backend: &AgentBackend, model: &str) -> String {
    let backend_str = backend_to_provider_str(backend);
    let sanitized_model = model
        .replace('/', "_")
        .replace(' ', "_")
        .replace(':', "_");
    if sanitized_model.is_empty() {
        format!("{prefix}_{}_{backend_str}", index + 1)
    } else {
        format!("{prefix}_{}_{backend_str}_{sanitized_model}", index + 1)
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

/// Pre-built file references for the enhanced prompt and plan artifacts.
pub struct IterationRefs {
    pub enhanced_ref: String,
    pub plan_ref: Option<String>,
}

/// Builds file references for the enhanced prompt and selected plan of the
/// current iteration. Used by reviewers, merger, fixer, and judge stages.
pub fn build_iteration_refs(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    iter_num: u32,
    enhanced: &str,
    iter_ctx: &crate::orchestrator::run_setup::IterationContext,
) -> IterationRefs {
    let enhanced_ref = file_ref_or_inline(
        workspace_path, session_id, run_id, iter_num, "enhanced_prompt", enhanced,
    );

    let plan_ref = iter_ctx.selected_plan().and_then(|_| {
        let kind = if iter_ctx.audited_plan.is_some() {
            "plan_audit"
        } else {
            "plan"
        };
        artifact_file_path(workspace_path, session_id, run_id, iter_num, kind)
            .map(|p| file_ref(&p))
    });

    IterationRefs {
        enhanced_ref,
        plan_ref,
    }
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
