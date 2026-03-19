//! Plan audit output parsing.

/// Parsed result from plan auditor output.
#[derive(Clone, Debug)]
pub struct PlanAuditParsed {
    pub improved_plan: String,
}

/// Parses plan auditor output.
///
/// The auditor now outputs the merged/improved plan directly without any
/// verdict prefix. This parser strips CLI noise and extracts the plan text.
/// If the output contains an `--- Improved Plan ---` marker (legacy format),
/// we extract the plan from after that marker.
pub fn parse_plan_audit_output(output: &str, fallback_plan: &str) -> PlanAuditParsed {
    let normalised = output.replace("\r\n", "\n");

    // Strip any legacy verdict lines that agents might still emit.
    let stripped = strip_legacy_verdict_prefix(&normalised);

    // Check for an explicit plan marker (legacy or edge-case).
    let marker = "--- Improved Plan ---";
    let alt_marker = "--- Rewritten Plan ---";
    let marker_pos = match (stripped.rfind(marker), stripped.rfind(alt_marker)) {
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

    let plan_candidate = if let Some((idx, marker_len)) = marker_pos {
        stripped[idx + marker_len..].trim().to_string()
    } else {
        stripped.trim().to_string()
    };

    let mut cleaned = strip_plan_tail_noise(&plan_candidate);
    if looks_like_template_noise(&cleaned) {
        cleaned.clear();
    }

    let improved = if cleaned.trim().is_empty() {
        fallback_plan.to_string()
    } else {
        cleaned
    };

    PlanAuditParsed { improved_plan: improved }
}

/// Strips legacy APPROVED/REJECTED lines from the start of the output.
fn strip_legacy_verdict_prefix(text: &str) -> String {
    let mut lines = text.lines().peekable();
    // Skip leading noise lines (codex preamble, blank lines, verdict lines).
    let mut skipped_verdict = false;
    let mut kept: Vec<&str> = Vec::new();
    for line in lines.by_ref() {
        let trimmed = line.trim();
        if !skipped_verdict {
            if trimmed.is_empty()
                || trimmed == "APPROVED"
                || trimmed == "REJECTED"
                || trimmed == "codex"
                || trimmed == "exec"
            {
                skipped_verdict = trimmed == "APPROVED" || trimmed == "REJECTED";
                continue;
            }
            // First non-noise line — start keeping.
            skipped_verdict = true;
        }
        kept.push(line);
    }
    kept.join("\n")
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
