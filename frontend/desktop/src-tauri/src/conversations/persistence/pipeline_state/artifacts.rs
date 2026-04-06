use std::path::PathBuf;

use crate::models::PipelineStageRecord;

use super::super::paths::conversation_dir;

/// Returns the expected artifact file path for a given pipeline stage,
/// or `None` if the stage type is unrecognised.
///
/// This is the single source of truth for stage → artifact mapping,
/// shared by both hydration (loading text into memory) and recovery
/// (reconciling stale running stages on startup).
pub(in crate::conversations::persistence) fn artifact_path_for_stage(
    workspace_path: &str,
    conversation_id: &str,
    stage: &PipelineStageRecord,
) -> Option<PathBuf> {
    let root = conversation_dir(workspace_path, conversation_id);

    if stage.stage_name == "Prompt Enhancer" {
        return Some(
            root.join("prompt_enhanced")
                .join("prompt_enhanced_output.json"),
        );
    }

    if stage.stage_name.starts_with("Planner") {
        let planner_number = stage
            .stage_name
            .strip_prefix("Planner ")
            .and_then(|n| {
                // Handle "(Cycle N)" suffix: "Planner 1 (Cycle 2)" → "1"
                n.split_whitespace().next()
            })
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(stage.stage_index + 1);
        return Some(root.join("plan").join(format!("Plan-{planner_number}.md")));
    }

    if stage.stage_name == "Plan Merge" {
        return Some(root.join("plan_merged").join("plan_merged.md"));
    }

    if stage.stage_name == "Coder" || stage.stage_name.starts_with("Coder") {
        return Some(root.join("coder").join("coder_done.md"));
    }

    if stage.stage_name.starts_with("Reviewer") {
        let reviewer_number = stage
            .stage_name
            .strip_prefix("Reviewer ")
            .and_then(|n| n.split_whitespace().next())
            .and_then(|n| n.parse::<usize>().ok());
        if let Some(idx) = reviewer_number {
            return Some(root.join("review").join(format!("Review-{idx}.md")));
        }
    }

    if stage.stage_name == "Review Merge" {
        return Some(root.join("review_merged").join("review_merged.md"));
    }

    if stage.stage_name == "Code Fixer" || stage.stage_name.starts_with("Code Fixer") {
        return Some(root.join("code_fixer").join("code_fixer_done.md"));
    }

    None
}
