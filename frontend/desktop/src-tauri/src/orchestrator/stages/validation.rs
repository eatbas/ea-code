//! Output validation for text-based agent stages.

use crate::models::PipelineStage;

pub fn validate_text_stage_output(stage: &PipelineStage, output: &str) -> Result<(), String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("agent returned empty output".to_string());
    }

    if looks_like_cli_result_envelope(trimmed) {
        return Err(
            "agent returned a CLI result envelope instead of the final artefact".to_string(),
        );
    }

    // Skip the preamble check for planning stages — planners naturally
    // narrate their exploration/reasoning process ("I'll start by…")
    // before producing the plan, so the check causes false-positive retries.
    if !matches!(stage, PipelineStage::Plan | PipelineStage::ExtraPlan(_)) {
        if looks_like_process_preamble(trimmed) {
            return Err(
                "agent returned a process preamble instead of the final artefact".to_string(),
            );
        }
    }

    match stage {
        PipelineStage::CodeReviewer
        | PipelineStage::ExtraReviewer(_)
        | PipelineStage::ReviewMerge => {
            if !trimmed.contains("## BLOCKERS") || !trimmed.contains("Verdict:") {
                return Err("review output did not match the required review schema".to_string());
            }
        }
        PipelineStage::Judge => {
            let first_non_empty = trimmed
                .lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("");
            let first_trimmed = first_non_empty.trim();
            let is_complete = first_trimmed.eq_ignore_ascii_case("COMPLETE");
            let is_not_complete = first_trimmed.eq_ignore_ascii_case("NOT COMPLETE");
            let has_verdict_line = trimmed.lines().any(|line| {
                let t = line.trim();
                t.len() >= 8 && t[..8].eq_ignore_ascii_case("VERDICT:")
            });

            if !is_complete && !is_not_complete && !has_verdict_line {
                return Err("judge output did not include a parseable verdict line".to_string());
            }
        }
        _ => {}
    }

    Ok(())
}

pub fn looks_like_cli_result_envelope(text: &str) -> bool {
    let compact = text.trim_start();
    compact.starts_with("{\"type\":\"result\"")
        || compact.starts_with("{\"subtype\":")
        || (compact.starts_with('{')
            && compact.contains("\"type\":\"result\"")
            && compact.contains("\"stop_reason\""))
}

/// Phrases that indicate the agent is narrating its process rather than
/// returning a final artefact. Stored as a static to avoid re-allocation.
pub static PREAMBLE_PREFIXES: &[&str] = &[
    "i'll start by",
    "i will start by",
    "i\u{2019}ll start by",
    "let me start by",
    "first, i'll",
    "first, i\u{2019}ll",
    "first i'll",
    "first i\u{2019}ll",
    "i'm going to start by",
    "i\u{2019}m going to start by",
    "i am going to start by",
];

pub fn looks_like_process_preamble(text: &str) -> bool {
    let first_non_empty = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    let lower = first_non_empty.to_lowercase();
    PREAMBLE_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{
        looks_like_cli_result_envelope, looks_like_process_preamble, validate_text_stage_output,
    };
    use crate::models::PipelineStage;

    #[test]
    fn rejects_cli_result_envelope() {
        let raw =
            r#"{"type":"result","subtype":"success","result":"hello","stop_reason":"end_turn"}"#;
        assert!(looks_like_cli_result_envelope(raw));
        assert!(validate_text_stage_output(&PipelineStage::ExtraPlan(1), raw).is_err());
    }

    #[test]
    fn rejects_process_preamble_for_non_planner_stages() {
        let raw = "I'll start by exploring the codebase to understand its structure.";
        assert!(looks_like_process_preamble(raw));
        // Non-planner stages should still reject preamble output.
        assert!(validate_text_stage_output(&PipelineStage::Coder, raw).is_err());
    }

    #[test]
    fn accepts_process_preamble_for_planner_stages() {
        let raw = "I'll start by exploring the codebase to understand its structure.\n\nHere is the plan...";
        assert!(looks_like_process_preamble(raw));
        // Planner stages naturally narrate their process — preamble is acceptable.
        assert!(validate_text_stage_output(&PipelineStage::Plan, raw).is_ok());
        assert!(validate_text_stage_output(&PipelineStage::ExtraPlan(0), raw).is_ok());
        assert!(validate_text_stage_output(&PipelineStage::ExtraPlan(1), raw).is_ok());
    }

    #[test]
    fn accepts_structured_review_output() {
        let raw = "## BLOCKERS\n- None.\n\n## WARNINGS\n- None.\n\n## NITS\n- None.\n\n## TESTS\n- Status: not run\n- Commands: None.\n\n## TEST RESULTS\n- None.\n\n## TEST GAPS\n- Add coverage.\n\n## ACTION ITEMS\n- None.\n\n## SUMMARY\nVerdict: PASS";
        assert!(validate_text_stage_output(&PipelineStage::CodeReviewer, raw).is_ok());
    }
}
