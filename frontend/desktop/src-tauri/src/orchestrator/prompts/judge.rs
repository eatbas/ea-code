//! Prompt builder for the Judge agent — the completion evaluator.

use crate::models::ReviewFindings;

use super::PromptMeta;

pub fn build_judge_system(meta: &PromptMeta) -> String {
    let is_final = meta.iteration == meta.max_iterations;
    let has_progress = meta.previous_judge_output.is_some();

    let mut parts = vec![format!(
        "# Role\n\
         You are the Judge agent in a multi-agent self-improving pipeline \
         (iteration {iter} of {max}).\n\
         You decide whether the development task is complete based on recent \
         code changes and the review summary.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - NEVER fix code yourself. You are NOT the Coder or Fixer.\n\
         - NEVER write code into source files or execute commands that change \
         the file system.\n\
         - You may use read-only tools (Read, Grep, git diff, git status) to \
         inspect the codebase.\n\
         - If an OUTPUT FILE path is provided at the end of the prompt, write \
         your verdict there. That is the ONLY file you may write.\n\
         \n\
         # Rubric\n\
         Evaluate against this checklist:\n\
         1. [REQUIRED] Does the diff satisfy the user's original prompt?\n\
         2. [REQUIRED] Are all BLOCKER-severity review items resolved? \
         (The review was run BEFORE the fixer — always verify BLOCKERs \
         against the current code, not just the review text.)\n\
         3. [REQUIRED] Are there any security issues?\n\
         4. [RECOMMENDED] Do new code paths have test coverage?\n\
         5. [RECOMMENDED] Does the code follow existing project conventions?\n\
         \n\
         If ALL required criteria pass, output COMPLETE.\n\
         If any required criterion fails, output NOT COMPLETE.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )];

    if has_progress {
        parts.push(
            "\n# Progress Awareness\n\
             You have judged a previous iteration. Your prior verdict is \
             included in the prompt as PREVIOUS JUDGE VERDICT.\n\
             - Focus on verifying whether previously flagged issues have \
             actually been fixed in the current code.\n\
             - Re-check each unchecked REQUIRED item from your prior verdict \
             against the current state of the code.\n\
             - Do NOT re-flag issues that have been resolved — mark them [x].\n\
             - Only flag items that are genuinely still present in the current code.\n\
             - If an issue was partially addressed and the remaining risk is \
             negligible, consider it resolved."
                .to_string(),
        );
    }

    if is_final {
        let mut final_section = "\n# Final Iteration Guidance\n\
             This is the last iteration. Apply a pragmatic standard:\n\
             - Minor NITs or RECOMMENDED items that are unresolved should NOT \
             prevent COMPLETE.\n\
             - Only block completion for genuine REQUIRED criterion failures."
            .to_string();
        if has_progress {
            final_section.push_str(
                "\n- If the code is substantially correct and only minor polish \
                 remains, output COMPLETE.\n\
                 - Partially addressed security items that pose no real-world risk \
                 in the current context should not block COMPLETE.",
            );
        }
        parts.push(final_section);
    }

    parts.push(
        "\n# Git Inspection\n\
         - Prefer inspecting recent changes directly via git before deciding \
         the verdict.\n\
         - Run: git status --short\n\
         - Run: git diff --cached --no-ext-diff --\n\
         - Run: git diff --no-ext-diff --\n\
         - Run: git ls-files --others --exclude-standard\n\
         - For each untracked file: git diff --no-index -- /dev/null <file-path>\n\
         - Use the review findings below as guidance, but always verify by \
         inspecting the actual code changes.\n\
         \n\
         # Output Format\n\
         Your response MUST begin with exactly one of these two lines \
         (no other text on the first line):\n\
         \n\
         COMPLETE\n\
         \n\
         or\n\
         \n\
         NOT COMPLETE\n\
         \n\
         Then provide:\n\
         \n\
         ## Checklist\n\
         - [x] or [ ] for each rubric item above\n\
         \n\
         ## Next Steps (only if NOT COMPLETE)\n\
         1. Exact step to resolve the issue.\n\
         2. Exact step...\n\
         \n\
         ## Handoff (only if NOT COMPLETE)\n\
         Provide a fenced JSON block for the next iteration using exactly \
         these fields:\n\
         ```json\n\
         {\n  \
           \"goal\": \"one-sentence task goal\",\n  \
           \"changes_summary\": \"what changed so far\",\n  \
           \"open_issues\": \"what is still unresolved\",\n  \
           \"next_actions\": \"1. step one\\n2. step two\",\n  \
           \"judge_required_items\": \"unchecked REQUIRED checklist items\"\n\
         }\n\
         ```\n\
         \n\
         # Context7 — Documentation Lookup\n\
         - When evaluating whether the code uses libraries or APIs correctly, \
         use the Context7 MCP tool to look up the latest documentation and \
         verify correctness.\n\
         - Always call `resolve-library-id` first to get the Context7-compatible \
         library ID, then call `get-library-docs` to fetch the documentation.\n\
         - Mark incorrect or deprecated API usage as a REQUIRED criterion failure.\n\
         \n\
         # Constraints\n\
         - Keep your response under 1000 tokens.\n\
         - Lead with the verdict. Do not embed the word \"complete\" in \
         explanatory text before the verdict line."
            .to_string(),
    );

    parts.join("\n")
}

pub fn build_judge_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plan: Option<&str>,
    findings: &ReviewFindings,
    previous_judge: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("--- Original Prompt ---\n{original_prompt}"),
        format!("--- Enhanced Prompt ---\n{enhanced_prompt}"),
    ];
    if let Some(p) = plan {
        parts.push(format!("--- Approved Plan ---\n{p}"));
    }

    // Format compact findings block
    let blockers_text = if findings.blockers.is_empty() {
        "None".to_string()
    } else {
        findings
            .blockers
            .iter()
            .map(|b| format!("  - {b}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let warnings_text = if findings.warnings.is_empty() {
        "None".to_string()
    } else {
        findings
            .warnings
            .iter()
            .map(|w| format!("  - {w}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let tests_str = if findings.tests_run { "run" } else { "not run" };

    parts.push(format!(
        "--- Review Findings ---\n\
         BLOCKERS: {}\n\
         {}\n\
         WARNINGS: {}\n\
         {}\n\
         TESTS: {}\n\
         VERDICT: {}",
        findings.blockers.len(),
        blockers_text,
        findings.warnings.len(),
        warnings_text,
        tests_str,
        findings.verdict
    ));

    if let Some(prev) = previous_judge {
        parts.push(format!("PREVIOUS JUDGE VERDICT:\n{prev}"));
    }

    parts.push(
        "Inspect repository changes using tools (especially git diff) \
         before final judgement.\n\
         First line must be COMPLETE or NOT COMPLETE."
            .to_string(),
    );
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judge_system_adds_final_iteration_guidance() {
        let meta = PromptMeta {
            iteration: 3,
            max_iterations: 3,
            previous_judge_output: Some("NOT COMPLETE".to_string()),
        };
        let system = build_judge_system(&meta);
        assert!(system.contains("Final Iteration Guidance"));
        assert!(system.contains("Progress Awareness"));
        assert!(system.contains("pragmatic standard"));
    }

    #[test]
    fn judge_system_omits_final_guidance_for_early_iteration() {
        let meta = PromptMeta {
            iteration: 1,
            max_iterations: 3,
            previous_judge_output: None,
        };
        let system = build_judge_system(&meta);
        assert!(!system.contains("Final Iteration Guidance"));
        assert!(!system.contains("Progress Awareness"));
    }

    #[test]
    fn judge_user_includes_previous_verdict() {
        let findings = ReviewFindings {
            blockers: vec!["Test blocker".to_string()],
            warnings: vec![],
            nits: vec![],
            tests_run: true,
            test_results: vec![],
            verdict: "FAIL".to_string(),
        };
        let user = build_judge_user(
            "task",
            "enhanced",
            None,
            &findings,
            Some("NOT COMPLETE\n## Checklist\n- [ ] blockers"),
        );
        assert!(user.contains("PREVIOUS JUDGE VERDICT"));
        assert!(user.contains("blockers"));
    }

    #[test]
    fn judge_user_includes_findings_block() {
        let findings = ReviewFindings {
            blockers: vec!["Missing validation".to_string()],
            warnings: vec!["Naming could improve".to_string()],
            nits: vec![],
            tests_run: false,
            test_results: vec![],
            verdict: "FAIL".to_string(),
        };
        let user = build_judge_user("task", "enhanced", None, &findings, None);
        assert!(user.contains("--- Review Findings ---"));
        assert!(user.contains("BLOCKERS: 1"));
        assert!(user.contains("Missing validation"));
        assert!(user.contains("WARNINGS: 1"));
        assert!(user.contains("Naming could improve"));
        assert!(user.contains("TESTS: not run"));
        assert!(user.contains("VERDICT: FAIL"));
    }

    #[test]
    fn judge_user_handles_empty_findings() {
        let findings = ReviewFindings {
            blockers: vec![],
            warnings: vec![],
            nits: vec![],
            tests_run: true,
            test_results: vec![],
            verdict: "PASS".to_string(),
        };
        let user = build_judge_user("task", "enhanced", None, &findings, None);
        assert!(user.contains("BLOCKERS: 0"));
        assert!(user.contains("None"));
        assert!(user.contains("VERDICT: PASS"));
    }
}
