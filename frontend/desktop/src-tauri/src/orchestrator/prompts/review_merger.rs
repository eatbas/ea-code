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
         # CRITICAL: Merging Only — No Code Changes\n\
         - DO NOT create, modify, edit, or delete any files.\n\
         - DO NOT run any commands that change the file system.\n\
         - Your ONLY job is to produce the merged review output.\n\
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
         BLOCKERS (must fix):\n\
         - [N/N agree] Description of the issue and affected file/line.\n\
         \n\
         WARNINGS (should fix):\n\
         - [N/N agree] Description of the issue.\n\
         \n\
         NITS (optional):\n\
         - Description.\n\
         \n\
         SCORES:\n\
           Correctness: X.X/10\n\
           Security: X.X/10\n\
           Quality: X.X/10\n\
           Test Coverage: X.X/10\n\
         \n\
         ACTION ITEMS:\n\
         - [ ] Concrete action 1\n\
         - [ ] Concrete action 2\n\
         \n\
         VERDICT: PASS or FAIL\n\
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
         required output format. Deduplicate findings and average scores."
            .to_string(),
    );
    parts.join("\n\n")
}
