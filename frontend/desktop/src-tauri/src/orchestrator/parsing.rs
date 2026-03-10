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
    let normalised = output.replace("\r\n", "\n");
    let lines: Vec<&str> = normalised.lines().collect();

    let mut verdict = "INVALID".to_string();
    let mut verdict_line_idx: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "APPROVED" || trimmed == "REJECTED" {
            verdict = trimmed.to_string();
            verdict_line_idx = Some(idx);
        }
    }

    let marker = "--- Improved Plan ---";
    let alt_marker = "--- Rewritten Plan ---";
    let after_verdict = if let Some(v_idx) = verdict_line_idx {
        lines[v_idx + 1..].join("\n")
    } else {
        normalised.clone()
    };
    let marker_pos = match (after_verdict.rfind(marker), after_verdict.rfind(alt_marker)) {
        (Some(a), Some(b)) => {
            if a >= b {
                Some((a, marker.len()))
            } else {
                Some((b, alt_marker.len()))
            }
        }
        (Some(a), None) => Some((a, marker.len())),
        (None, Some(b)) => Some((b, alt_marker.len())),
        (None, None) => None,
    };

    let reasoning_raw = if let Some((idx, _)) = marker_pos {
        after_verdict[..idx].trim().to_string()
    } else if verdict == "REJECTED" {
        after_verdict.trim().to_string()
    } else {
        String::new()
    };

    let plan_candidate = if let Some((idx, marker_len)) = marker_pos {
        after_verdict[idx + marker_len..].trim().to_string()
    } else if verdict == "REJECTED" {
        String::new()
    } else if let Some(v_idx) = verdict_line_idx {
        lines[v_idx + 1..].join("\n").trim().to_string()
    } else {
        normalised.trim().to_string()
    };

    let mut cleaned_plan = strip_plan_tail_noise(&plan_candidate);
    if looks_like_template_noise(&cleaned_plan) {
        cleaned_plan.clear();
    }
    let improved = if cleaned_plan.trim().is_empty() {
        if verdict == "REJECTED" {
            fallback_plan.to_string()
        } else if verdict == "INVALID" {
            fallback_plan.to_string()
        } else if !plan_candidate.trim().is_empty() {
            plan_candidate.trim().to_string()
        } else {
            fallback_plan.to_string()
        }
    } else {
        cleaned_plan
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

fn strip_plan_tail_noise(text: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        let is_noise_boundary = lower.starts_with("tokens used")
            || lower.starts_with("total tokens")
            || lower.starts_with("total cost")
            || lower.starts_with("total duration")
            || lower == "exec"
            || lower == "codex"
            || lower.starts_with("<image>")
            || (trimmed.starts_with('"')
                && lower.contains("powershell")
                && lower.contains(".exe"))
            || lower.starts_with("succeeded in ");

        if is_noise_boundary {
            break;
        }
        kept.push(line);
    }

    kept.join("\n").trim().to_string()
}

fn looks_like_template_noise(text: &str) -> bool {
    let preview = text.trim().to_ascii_lowercase();
    let preview = preview.chars().take(400).collect::<String>();
    if preview.is_empty() {
        return false;
    }
    if preview.starts_with("# inputs") && preview.contains("# output constraints") {
        return true;
    }
    if preview.starts_with("--- workspace context ---")
        || preview.starts_with("workspace snapshot")
        || preview.starts_with("worktree snapshot")
    {
        return true;
    }
    false
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
}
