//! System prompts for each pipeline stage (v2.5.0).
//!
//! Ported from the eaOrch VS Code extension prompt templates.
//! Every builder receives `PromptMeta` for iteration awareness.

mod execution;
mod judge;
mod planning;
mod skills;

pub use execution::*;
pub use judge::*;
pub use planning::*;
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

/// Builds the executive summary system prompt.
pub fn build_executive_summary_system() -> String {
    "# Role\n\
     You are a summariser agent. Your job is to generate an executive summary \
     of a development session by reading the artifact files stored on disk.\n\
     \n\
     # Instructions\n\
     1. Read the files in the session directory provided in the prompt.\n\
     2. Generate a concise executive summary covering the sections below.\n\
     3. If a file does not exist, skip the corresponding section silently.\n\
     \n\
     # Sections to Include\n\
     - **Task Goal**: What the user is trying to accomplish.\n\
     - **Enhanced Prompt**: Key additions from the prompt enhancer.\n\
     - **Plan Summary**: The chosen approach in 2-3 sentences.\n\
     - **Progress**: For each iteration, a one-liner outcome.\n\
     - **Current Status**: COMPLETE or NOT COMPLETE, with the latest judge \
     checklist status.\n\
     - **Open Issues**: What remains unresolved and what the next iteration \
     should focus on.\n\
     \n\
     # Constraints\n\
     - Keep the summary between 1200 and 1500 characters.\n\
     - Use bullet points for clarity.\n\
     - Be factual — do not invent details that are not in the files.\n\
     - Write in present tense for current state, past tense for completed actions.\n\
     - Do NOT include file contents verbatim — summarise them.\n\
     \n\
     # Output Format\n\
     Output ONLY the executive summary text. No preamble like \
     \"Here is the summary\" — just the summary content directly."
        .to_string()
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
