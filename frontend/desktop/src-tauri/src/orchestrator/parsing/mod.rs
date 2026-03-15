//! Verdict and plan audit output parsing.

use crate::models::JudgeVerdict;

mod plan;
mod reviewer;

pub use plan::parse_plan_audit_output;
pub use reviewer::parse_review_findings;

/// Parses the judge verdict from raw output text.
///
/// Uses a three-tier strategy:
/// 1. Exact first-line match
/// 2. Checklist heuristic (unchecked REQUIRED items)
/// 3. Keyword heuristic (fail-safe to NOT COMPLETE)
pub fn parse_judge_verdict(output: &str) -> (JudgeVerdict, String) {
    let first_line = output.lines().next().unwrap_or("").trim();
    let reasoning = output.lines().skip(1).collect::<Vec<_>>().join("\n");

    // Tier 1: Exact first-line match
    if first_line == "COMPLETE" {
        return (JudgeVerdict::Complete, reasoning);
    }
    if first_line == "NOT COMPLETE" {
        return (JudgeVerdict::NotComplete, reasoning);
    }

    // Tier 2: Checklist heuristic — any unchecked [REQUIRED] forces NOT COMPLETE
    if let Some(checklist_start) = output.find("## Checklist") {
        let checklist = &output[checklist_start..];
        let has_unchecked_required = checklist
            .lines()
            .any(|l| l.contains("[ ]") && l.contains("[REQUIRED]"));
        if has_unchecked_required {
            return (JudgeVerdict::NotComplete, reasoning);
        }
        let has_checked_required = checklist
            .lines()
            .any(|l| l.contains("[x]") && l.contains("[REQUIRED]"));
        if has_checked_required {
            return (JudgeVerdict::Complete, reasoning);
        }
    }

    // Tier 3: Keyword heuristic in first 3 lines
    let first_three: String = output
        .lines()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let positive = ["complete", "done", "ship it", "ready to merge"];
    let negative = ["incomplete", "missing", "issues", "fails", "not complete"];

    let has_positive = positive.iter().any(|kw| first_three.contains(kw));
    let has_negative = negative.iter().any(|kw| first_three.contains(kw));

    if has_negative || !has_positive {
        (JudgeVerdict::NotComplete, reasoning)
    } else {
        (JudgeVerdict::Complete, reasoning)
    }
}

/// Extracts a `[QUESTION]...[/QUESTION]` block from agent output.
pub fn extract_question(output: &str) -> Option<String> {
    let start_tag = "[QUESTION]";
    let end_tag = "[/QUESTION]";
    if let Some(start) = output.find(start_tag) {
        if let Some(end) = output.find(end_tag) {
            let question = output[start + start_tag.len()..end].trim();
            if !question.is_empty() {
                return Some(question.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_audit_approved_with_improved_plan() {
        let raw = "APPROVED\nLooks good.\n--- Improved Plan ---\n1. Do A\n2. Do B";
        let parsed = parse_plan_audit_output(raw, "fallback");
        assert_eq!(parsed.verdict, "APPROVED");
        assert_eq!(parsed.improved_plan, "1. Do A\n2. Do B");
    }

    #[test]
    fn parse_plan_audit_rejected_with_rewrite_continues() {
        let raw = "REJECTED\nMissing checks.\n--- Improved Plan ---\n1. Add checks";
        let parsed = parse_plan_audit_output(raw, "fallback");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "1. Add checks");
    }

    #[test]
    fn parse_plan_audit_rejected_without_rewrite_uses_fallback() {
        let raw = "REJECTED\nNo rewrite provided.";
        let parsed = parse_plan_audit_output(raw, "fallback plan");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "fallback plan");
    }

    #[test]
    fn parse_plan_audit_noisy_codex_output_extracts_plan_only() {
        let raw = "codex\n\
I am auditing now.\n\
exec\n\
\"C:\\\\WINDOWS\\\\System32\\\\WindowsPowerShell\\\\v1.0\\\\powershell.exe\" -Command 'rg --files'\n\
REJECTED\n\
--- Improved Plan ---\n\
1. Update parsing.\n\
2. Add tests.\n\
tokens used\n\
27,719";
        let parsed = parse_plan_audit_output(raw, "fallback");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "1. Update parsing.\n2. Add tests.");
    }

    #[test]
    fn parse_plan_audit_verdict_not_first_line_is_detected() {
        let raw = "codex\n\
REJECTED\n\
--- Improved Plan ---\n\
1. Rewrite for clarity.";
        let parsed = parse_plan_audit_output(raw, "fallback");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "1. Rewrite for clarity.");
    }

    #[test]
    fn parse_plan_audit_template_echo_uses_fallback_plan() {
        let raw = "REJECTED\n\
--- Improved Plan ---\n\
# Inputs\n\
- You may receive original prompt, enhanced prompt, planner draft.\n\
\n\
# Output Constraints\n\
- Return only the audited final plan text.\n\
- No markdown fences.\n\
\n\
--- Workspace Context ---\n\
WORKSPACE SNAPSHOT";
        let parsed = parse_plan_audit_output(raw, "fallback plan");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "fallback plan");
    }

    #[test]
    fn parse_plan_audit_marker_before_verdict_is_ignored() {
        let raw = "--- Improved Plan ---\n\
# Inputs\n\
REJECTED\n\
--- Improved Plan ---\n\
1. Correct final plan.";
        let parsed = parse_plan_audit_output(raw, "fallback plan");
        assert_eq!(parsed.verdict, "REJECTED");
        assert_eq!(parsed.improved_plan, "1. Correct final plan.");
    }

    #[test]
    fn parse_judge_exact_complete() {
        let (v, _) = parse_judge_verdict("COMPLETE\n## Checklist\n- [x] All good");
        assert_eq!(v, JudgeVerdict::Complete);
    }

    #[test]
    fn parse_judge_exact_not_complete() {
        let (v, _) = parse_judge_verdict("NOT COMPLETE\n## Checklist\n- [ ] Missing");
        assert_eq!(v, JudgeVerdict::NotComplete);
    }

    #[test]
    fn parse_judge_checklist_heuristic() {
        let output = "Some text\n## Checklist\n- [x] [REQUIRED] Prompt\n- [ ] [REQUIRED] Blockers";
        let (v, _) = parse_judge_verdict(output);
        assert_eq!(v, JudgeVerdict::NotComplete);
    }

    #[test]
    fn parse_judge_ambiguous_defaults_not_complete() {
        let (v, _) = parse_judge_verdict("The task has some issues remaining.");
        assert_eq!(v, JudgeVerdict::NotComplete);
    }

    #[test]
    fn parse_review_findings_extracts_blockers() {
        let raw = "BLOCKER: Missing input validation on line 45\nSome other text\nBLOCKER: No error handling for null pointer";
        let findings = parse_review_findings(raw);
        assert_eq!(findings.blockers.len(), 2);
        assert!(findings.blockers[0].contains("input validation"));
        assert!(findings.blockers[1].contains("error handling"));
    }

    #[test]
    fn parse_review_findings_extracts_warnings() {
        let raw = "WARNING: Token expiry hardcoded\nWARNING: Variable name unclear";
        let findings = parse_review_findings(raw);
        assert_eq!(findings.warnings.len(), 2);
        assert!(findings.warnings[0].contains("expiry"));
    }

    #[test]
    fn parse_review_findings_extracts_tests_and_verdict() {
        let raw = "BLOCKER: Issue found\nTESTS: run\nVERDICT: PASS";
        let findings = parse_review_findings(raw);
        assert!(findings.tests_run);
        assert_eq!(findings.verdict, "PASS");
    }

    #[test]
    fn parse_review_findings_infer_fail_from_blockers() {
        let raw = "BLOCKER: Critical issue found\nSome other text";
        let findings = parse_review_findings(raw);
        assert_eq!(findings.verdict, "FAIL");
    }

    #[test]
    fn parse_review_findings_infer_pass_when_no_blockers() {
        let raw = "LGTM! Looks good to me. No blockers found.";
        let findings = parse_review_findings(raw);
        assert!(findings.blockers.is_empty());
        assert_eq!(findings.verdict, "PASS");
    }
}
