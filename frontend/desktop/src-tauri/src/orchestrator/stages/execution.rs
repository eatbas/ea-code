//! Stage execution helper functions.

use crate::models::{RunEvent, StageEndStatus, *};
use crate::storage::{self, runs};

/// Helper to append stage_end events to the event log.
pub fn append_stage_end_event(
    run_id: &str,
    stage: &str,
    iteration: u32,
    duration_ms: u64,
    status: &str,
) {
    let seq = match runs::next_sequence(run_id) {
        Ok(s) => s,
        Err(_) => 1,
    };

    let stage_end_status = match status {
        "completed" => StageEndStatus::Completed,
        "failed" => StageEndStatus::Failed,
        "skipped" => StageEndStatus::Skipped,
        _ => StageEndStatus::Completed,
    };

    // Map string stage to PipelineStage for the event
    let stage_enum = match stage {
        "prompt_enhance" => PipelineStage::PromptEnhance,
        "skill_select" => PipelineStage::SkillSelect,
        "plan" => PipelineStage::Plan,
        "plan_audit" => PipelineStage::PlanAudit,
        "coder" => PipelineStage::Coder,
        "code_reviewer" => PipelineStage::CodeReviewer,
        "code_fixer" => PipelineStage::CodeFixer,
        "judge" => PipelineStage::Judge,
        "executive_summary" => PipelineStage::ExecutiveSummary,
        "direct_task" => PipelineStage::DirectTask,
        _ => PipelineStage::DirectTask,
    };

    let event = RunEvent::StageEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage_enum,
        iteration,
        status: stage_end_status,
        duration_ms,
        verdict: None,
        input_tokens: None,
        output_tokens: None,
        estimated_cost_usd: None,
        session_pair: None,
        resumed: None,
    };

    if let Err(e) = runs::append_event(run_id, event) {
        eprintln!("Warning: Failed to append stage event: {e}");
    }
}

/// Serialises a PipelineStage to its snake_case string.
pub fn stage_to_str(stage: &PipelineStage) -> String {
    serde_json::to_value(stage)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{stage:?}"))
}
