//! Reviewer output parsing.

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

    if verdict == "FAIL" && blockers.is_empty() && reviewer_output.to_ascii_lowercase().contains("verdict: pass") {
        verdict = "PASS".to_string();
    }

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
    if !item.is_empty() && !item.eq_ignore_ascii_case("none") {
        target.push(item.to_string());
    }
    true
}
