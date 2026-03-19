//! System prompts for each pipeline stage (v2.5.0).
//!
//! Ported from the eaOrch VS Code extension prompt templates.
//! Every builder receives `PromptMeta` for iteration awareness.

mod enhancer;
mod executive_summary;
mod fixer;
mod generator;
mod judge;
mod plan_auditor;
mod planner;
mod review_merger;
mod reviewer;
mod skills;

pub use enhancer::*;
pub use executive_summary::*;
pub use fixer::*;
pub use generator::*;
pub use judge::*;
pub use plan_auditor::*;
pub use planner::*;
pub use review_merger::*;
pub use reviewer::*;
pub use skills::*;

/// Metadata injected into every prompt builder so agents are iteration-aware.
#[derive(Clone, Debug)]
pub struct PromptMeta {
    /// Current iteration number (1-based).
    pub iteration: u32,
    /// Maximum iterations configured for this run.
    pub max_iterations: u32,
    /// Previous judge output for progress tracking (iteration 2+ only).
    pub previous_judge_output: Option<String>,
}

/// Structured handoff data passed between iterations by the judge.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IterationHandoff {
    pub goal: String,
    pub changes_summary: String,
    pub open_issues: String,
    pub next_actions: String,
    pub judge_required_items: String,
}

/// Renders a compact handoff block suitable for injection into later stages.
pub fn render_handoff_for_prompt(handoff: &IterationHandoff) -> String {
    [
        "PRIOR ITERATION HANDOFF:".to_string(),
        format!("Goal: {}", handoff.goal.trim()),
        format!("Progress Summary: {}", handoff.changes_summary.trim()),
        format!("Open Issues: {}", handoff.open_issues.trim()),
        format!("Required Unresolved Items: {}", handoff.judge_required_items.trim()),
        format!("Next Actions:\n{}", handoff.next_actions.trim()),
    ]
    .join("\n")
}

/// Formats a list of items as indented bullet lines, or "None" if empty.
pub fn format_indented_bullet_list(items: &[String]) -> String {
    if items.is_empty() {
        "  - None".to_string()
    } else {
        items
            .iter()
            .map(|item| format!("  - {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Truncates the previous judge output to a reasonable size for injection.
pub fn truncate_judge_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        output.to_string()
    } else {
        format!("{}…[truncated]", &output[..max_chars])
    }
}

/// Attempts to parse a structured `IterationHandoff` from judge output.
///
/// Looks for a fenced JSON block under `## Handoff`.
pub fn parse_handoff(judge_output: &str) -> Option<IterationHandoff> {
    let handoff_marker = "## Handoff";
    let handoff_start = judge_output.find(handoff_marker)?;
    let remainder = &judge_output[handoff_start..];

    let json_start = remainder.find("```json").or_else(|| remainder.find("```"))?;
    let after_fence = &remainder[json_start..];
    let content_start = after_fence.find('\n')? + 1;
    let content_end = after_fence[content_start..].find("```")?;
    let json_str = &after_fence[content_start..content_start + content_end].trim();

    serde_json::from_str(json_str).ok()
}

/// Builds a fallback handoff when JSON parsing fails.
pub fn build_fallback_handoff(
    task_brief: &str,
    judge_output: &str,
    iteration: u32,
) -> IterationHandoff {
    let next_steps = judge_output
        .find("## Next Steps")
        .map(|idx| {
            let remainder = &judge_output[idx..];
            remainder
                .lines()
                .skip(1)
                .take_while(|l| !l.starts_with("## "))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .unwrap_or_default();

    let required_items = judge_output
        .find("## Checklist")
        .map(|idx| {
            let remainder = &judge_output[idx..];
            remainder
                .lines()
                .skip(1)
                .take_while(|l| !l.starts_with("## "))
                .filter(|l| l.contains("[ ]") && l.contains("[REQUIRED]"))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .unwrap_or_default();

    IterationHandoff {
        goal: task_brief.chars().take(200).collect(),
        changes_summary: format!("Iteration {iteration} did not pass judge evaluation."),
        open_issues: required_items.clone(),
        next_actions: next_steps,
        judge_required_items: required_items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_handoff_from_judge_output() {
        let output = "NOT COMPLETE\n\n\
            ## Checklist\n\
            - [x] [REQUIRED] Does the diff satisfy the prompt?\n\
            - [ ] [REQUIRED] All BLOCKERs resolved?\n\
            \n\
            ## Next Steps\n\
            1. Fix the blocker.\n\
            \n\
            ## Handoff\n\
            ```json\n\
            {\n  \
              \"goal\": \"Add login form\",\n  \
              \"changes_summary\": \"Created form component\",\n  \
              \"open_issues\": \"Validation missing\",\n  \
              \"next_actions\": \"1. Add validation\",\n  \
              \"judge_required_items\": \"BLOCKERs not resolved\"\n\
            }\n\
            ```";
        let handoff = parse_handoff(output).expect("should parse handoff");
        assert_eq!(handoff.goal, "Add login form");
        assert_eq!(handoff.open_issues, "Validation missing");
    }

    #[test]
    fn parse_handoff_missing_returns_none() {
        let output = "COMPLETE\n\n## Checklist\n- [x] All good.";
        assert!(parse_handoff(output).is_none());
    }

    #[test]
    fn fallback_handoff_extracts_required_items() {
        let output = "NOT COMPLETE\n\n\
            ## Checklist\n\
            - [x] [REQUIRED] Prompt satisfied\n\
            - [ ] [REQUIRED] BLOCKERs resolved\n\
            - [ ] [RECOMMENDED] Tests added\n\
            \n\
            ## Next Steps\n\
            1. Fix blockers\n\
            2. Add tests";
        let handoff = build_fallback_handoff("Add feature", output, 1);
        assert!(handoff.judge_required_items.contains("BLOCKERs resolved"));
        assert!(!handoff.judge_required_items.contains("Tests added"));
        assert!(handoff.next_actions.contains("Fix blockers"));
    }

    #[test]
    fn truncate_judge_output_respects_limit() {
        let long = "a".repeat(5000);
        let truncated = truncate_judge_output(&long, 3000);
        assert!(truncated.len() < 3020);
        assert!(truncated.ends_with("…[truncated]"));
    }
}
