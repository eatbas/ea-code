//! Plan audit output parsing.

/// Parsed result from plan auditor output.
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
