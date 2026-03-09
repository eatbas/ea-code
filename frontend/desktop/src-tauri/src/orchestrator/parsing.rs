//! Verdict and plan audit output parsing.

use crate::models::JudgeVerdict;

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

#[derive(Clone, Debug)]
pub struct PlanAuditParsed {
    pub verdict: String,
    pub reasoning: String,
    pub improved_plan: String,
}

/// Parses plan auditor output.
/// Expected shape: line 1 = APPROVED or REJECTED, then optional
/// reasoning and `--- Improved Plan ---` section.
pub fn parse_plan_audit_output(output: &str, fallback_plan: &str) -> PlanAuditParsed {
    let mut lines = output.lines();
    let first_line = lines.next().unwrap_or("").trim();
    let mut verdict = if first_line == "APPROVED" || first_line == "REJECTED" {
        first_line.to_string()
    } else {
        "INVALID".to_string()
    };

    let remainder = lines.collect::<Vec<_>>().join("\n");
    let marker = "--- Improved Plan ---";
    let alt_marker = "--- Rewritten Plan ---";

    let (reasoning_raw, plan_raw) = if let Some(idx) = remainder.find(marker) {
        let (head, tail) = remainder.split_at(idx);
        (
            head.trim().to_string(),
            tail[marker.len()..].trim().to_string(),
        )
    } else if let Some(idx) = remainder.find(alt_marker) {
        let (head, tail) = remainder.split_at(idx);
        (
            head.trim().to_string(),
            tail[alt_marker.len()..].trim().to_string(),
        )
    } else if verdict == "REJECTED" {
        (remainder.trim().to_string(), String::new())
    } else {
        (String::new(), remainder.trim().to_string())
    };

    let improved = if plan_raw.trim().is_empty() {
        if verdict == "REJECTED" {
            fallback_plan.to_string()
        } else if !remainder.trim().is_empty() {
            remainder.trim().to_string()
        } else {
            fallback_plan.to_string()
        }
    } else {
        plan_raw
    };

    if verdict == "INVALID" && improved.trim().is_empty() {
        verdict = "REJECTED".to_string();
    }

    PlanAuditParsed {
        verdict,
        reasoning: reasoning_raw,
        improved_plan: improved,
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
}
