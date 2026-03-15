//! Reviewer output parsing.

use crate::models::ReviewFindings;

/// Parses reviewer output into compact structured review findings.
///
/// Extracts:
/// - BLOCKER lines (lines starting with "BLOCKER:" or "BLOCKER ")
/// - WARNING lines (lines starting with "WARNING:" or "WARNING ")
/// - Tests run status (looks for "TESTS: run" or "TESTS: not run")
/// - Verdict (looks for "VERDICT: PASS" or "VERDICT: FAIL")
pub fn parse_review_findings(reviewer_output: &str) -> ReviewFindings {
    let mut blockers: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut tests_run = false;
    let mut verdict = "FAIL".to_string();

    for line in reviewer_output.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();

        // Parse BLOCKER lines
        if lower.starts_with("blocker:") || lower.starts_with("blocker ") {
            let content = trimmed
                .trim_start_matches("BLOCKER:")
                .trim_start_matches("BLOCKER")
                .trim_start_matches(':')
                .trim();
            if !content.is_empty() {
                blockers.push(content.to_string());
            }
        }

        // Parse WARNING lines
        if lower.starts_with("warning:") || lower.starts_with("warning ") {
            let content = trimmed
                .trim_start_matches("WARNING:")
                .trim_start_matches("WARNING")
                .trim_start_matches(':')
                .trim();
            if !content.is_empty() {
                warnings.push(content.to_string());
            }
        }

        // Parse TESTS line
        if lower.starts_with("tests:") {
            let content = trimmed
                .trim_start_matches("TESTS:")
                .trim_start_matches("tests:")
                .trim()
                .to_ascii_lowercase();
            tests_run = content.starts_with("run") || content.starts_with("yes");
        }

        // Parse VERDICT line
        if lower.starts_with("verdict:") {
            let content = trimmed
                .trim_start_matches("VERDICT:")
                .trim_start_matches("verdict:")
                .trim()
                .to_ascii_uppercase();
            if content.starts_with("PASS") || content.starts_with("APPROVED") {
                verdict = "PASS".to_string();
            } else {
                verdict = "FAIL".to_string();
            }
        }
    }

    // If no explicit verdict found, infer from blockers
    if verdict == "FAIL" && blockers.is_empty() {
        // Check if there are any blocking keywords in the output
        let lower_output = reviewer_output.to_ascii_lowercase();
        if !lower_output.contains("verdict:")
            && !lower_output.contains("all good")
            && !lower_output.contains("no issues")
        {
            // Default to FAIL if we see blockers mentioned anywhere
            if lower_output.contains("blocker") || lower_output.contains("critical") {
                verdict = "FAIL".to_string();
            }
        }
    }

    // If we have no blockers and the output looks positive, assume PASS
    if blockers.is_empty() && verdict == "FAIL" {
        let lower_output = reviewer_output.to_ascii_lowercase();
        if lower_output.contains("lgtm")
            || lower_output.contains("looks good")
            || lower_output.contains("no blockers")
            || lower_output.contains("all checks passed")
        {
            verdict = "PASS".to_string();
        }
    }

    ReviewFindings {
        blockers,
        warnings,
        tests_run,
        verdict,
        nits: Vec::new(),
        test_results: Vec::new(),
    }
}
