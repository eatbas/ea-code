//! Reviewer output parsing.

use std::collections::HashSet;

use crate::models::ReviewFindings;

/// Parses reviewer or review-merger markdown output into structured findings.
pub fn parse_review_findings(reviewer_output: &str) -> ReviewFindings {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut nits = Vec::new();
    let mut tests_run = false;
    let mut test_commands = Vec::new();
    let mut test_results = Vec::new();
    let mut test_gaps = Vec::new();
    let mut verdict = "FAIL".to_string();
    let mut section = String::new();
    let mut in_test_commands = false;

    for raw_line in reviewer_output.lines() {
        let trimmed = raw_line.trim();
        let lower = trimmed.to_ascii_lowercase();

        if lower.starts_with("## ") {
            section = lower.trim_start_matches("## ").trim().to_string();
            in_test_commands = false;
            continue;
        }

        match section.as_str() {
            "blockers" => {
                let _ = push_bullet(trimmed, &mut blockers);
            }
            "warnings" => {
                let _ = push_bullet(trimmed, &mut warnings);
            }
            "nits" => {
                let _ = push_bullet(trimmed, &mut nits);
            }
            "tests" => {
                if lower.starts_with("status:") {
                    let status = trimmed
                        .trim_start_matches("Status:")
                        .trim_start_matches("status:")
                        .trim()
                        .to_ascii_lowercase();
                    tests_run = status.starts_with("run");
                    in_test_commands = false;
                } else if lower == "commands:" {
                    in_test_commands = true;
                } else if in_test_commands {
                    if !push_bullet(trimmed, &mut test_commands) && !trimmed.is_empty() {
                        in_test_commands = false;
                    }
                }
            }
            "test results" => {
                push_bullet(trimmed, &mut test_results);
            }
            "test gaps" => {
                push_bullet(trimmed, &mut test_gaps);
            }
            "summary" => {
                if lower.starts_with("verdict:") {
                    let content = trimmed
                        .trim_start_matches("Verdict:")
                        .trim_start_matches("verdict:")
                        .trim()
                        .to_ascii_uppercase();
                    verdict = if content.starts_with("PASS") {
                        "PASS".to_string()
                    } else {
                        "FAIL".to_string()
                    };
                }
            }
            _ => {}
        }
    }

    if verdict == "FAIL"
        && blockers.is_empty()
        && reviewer_output
            .to_ascii_lowercase()
            .contains("verdict: pass")
    {
        verdict = "PASS".to_string();
    }

    normalise_findings(&mut blockers, &mut warnings);

    ReviewFindings {
        blockers,
        warnings,
        nits,
        tests_run,
        test_commands,
        test_results,
        test_gaps,
        verdict,
    }
}

fn push_bullet(line: &str, target: &mut Vec<String>) -> bool {
    let Some(item) = line.strip_prefix("- ") else {
        return false;
    };
    let item = item.trim();
    if !item.is_empty() && !is_placeholder_item(item) {
        target.push(item.to_string());
    }
    true
}

fn is_placeholder_item(item: &str) -> bool {
    let normalised = item
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_ascii_lowercase();
    matches!(
        normalised.as_str(),
        "none" | "n/a" | "na" | "no blockers" | "no warnings" | "no nits"
    )
}

fn normalise_findings(blockers: &mut Vec<String>, warnings: &mut Vec<String>) {
    let mut kept_blockers = Vec::new();
    let mut demoted_warnings = Vec::new();

    for blocker in blockers.drain(..) {
        if let Some((agree, total)) = parse_consensus_marker(&blocker) {
            // Prevent single-reviewer or split opinions from forcing a hard blocker.
            if total > 1 && agree.saturating_mul(2) <= total {
                demoted_warnings.push(format!(
                    "{blocker} (demoted: no majority reviewer consensus)"
                ));
                continue;
            }
        }
        kept_blockers.push(blocker);
    }

    warnings.extend(demoted_warnings);
    *blockers = dedupe_preserve_order(kept_blockers);
    *warnings = dedupe_preserve_order(std::mem::take(warnings));
}

fn dedupe_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for item in items {
        let key = item.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(item);
        }
    }

    deduped
}

fn parse_consensus_marker(item: &str) -> Option<(u32, u32)> {
    let trimmed = item.trim_start();
    if !trimmed.starts_with('[') {
        return None;
    }
    let marker_end = trimmed.find(']')?;
    if marker_end <= 1 {
        return None;
    }
    let marker = trimmed.get(1..marker_end)?.trim();
    let ratio_token = marker.split_whitespace().next()?;
    let (agree_raw, total_raw) = ratio_token.split_once('/')?;
    let agree = agree_raw.parse::<u32>().ok()?;
    let total = total_raw.parse::<u32>().ok()?;

    if total == 0 || agree > total {
        return None;
    }

    Some((agree, total))
}
