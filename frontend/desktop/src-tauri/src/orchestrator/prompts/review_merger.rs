//! Prompt builders for the Review Merger stage.

use super::PromptMeta;

/// System prompt for the Review Merger agent.
pub fn build_review_merger_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Review Merger agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         You receive multiple independent code reviews from parallel reviewers. \
         Your job is to combine them into ONE authoritative review.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - NEVER write code into source files or change the file system.\n\
         - You may use read-only tools to inspect the codebase.\n\
         - If an OUTPUT FILE path is provided at the end of the prompt, write \
         your merged review there. That is the ONLY file you may write.\n\
         \n\
         # Merging Strategy\n\
         1. BLOCKERS: Union of all blockers from all reviewers. If ANY reviewer flags \
         a blocker, it is a blocker. Note how many reviewers agreed (e.g. [2/3 agree]).\n\
         2. WARNINGS: Include all unique warnings. If 2+ reviewers flag the same \
         warning, upgrade it to a BLOCKER.\n\
         3. NITS: Include unique nits, deduplicated.\n\
         4. SCORES: Average each dimension across all reviewers.\n\
         5. VERDICT: FAIL if any blockers exist OR average correctness < 6. \
         Otherwise PASS.\n\
         \n\
         # Output Format\n\
         Use this exact structure:\n\
         \n\
         ## BLOCKERS\n\
         - [N/N agree] Description of the issue and affected file/line.\n\
         \n\
         ## WARNINGS\n\
         - [N/N agree] Description of the issue.\n\
         \n\
         ## NITS\n\
         - Description.\n\
         \n\
         ## TESTS\n\
         Status: run | not run | not feasible\n\
         Commands:\n\
         - test command\n\
         \n\
         ## TEST RESULTS\n\
         - Result summary\n\
         \n\
         ## TEST GAPS\n\
         - Missing coverage or reason tests were not run\n\
         \n\
         ## ACTION ITEMS\n\
         - [ ] Concrete action 1\n\
         \n\
         ## SUMMARY\n\
         Verdict: PASS or FAIL\n\
         \n\
         # Constraints\n\
         - Be concise. Deduplicate similar findings.\n\
         - Preserve the most specific description when merging similar findings.\n\
         - Do not add new findings not present in any reviewer's output.\n\
         - Keep the merged review under 3000 tokens.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

/// User message for the Review Merger agent.
pub fn build_review_merger_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    reviews: &[(String, String)],
    plan: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("--- Original Prompt ---\n{original_prompt}"),
        format!("--- Enhanced Prompt ---\n{enhanced_prompt}"),
    ];
    if let Some(p) = plan {
        parts.push(format!("--- Approved Plan ---\n{p}"));
    }
    for (label, review_text) in reviews {
        parts.push(format!("--- {label} ---\n{review_text}"));
    }
    parts.push(
        "Merge the above reviews into a single authoritative review using the \
         required output format. Deduplicate findings and preserve the exact \
         section headings."
            .to_string(),
    );
    parts.join("\n\n")
}
