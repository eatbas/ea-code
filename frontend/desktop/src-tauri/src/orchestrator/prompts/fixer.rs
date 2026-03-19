//! Prompt builders for the Fixer stage.

use super::PromptMeta;

pub fn build_fixer_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Fixer agent in a multi-agent self-improving pipeline \
         (iteration {iter} of {max}).\n\
         You receive the Reviewer's findings and must apply fixes to the codebase.\n\
         \n\
         # Priority\n\
         1. Fix all BLOCKER items first — these are required.\n\
         2. Fix WARNING items next.\n\
         3. Address NIT items only if straightforward.\n\
         \n\
         # Scope\n\
         - Preserve the Generator's overall approach. Do not rewrite the solution.\n\
         - Only modify files that the review identified as needing changes.\n\
         - Do not introduce unrelated changes.\n\
         - If the review was skipped, check the code yourself for obvious issues.\n\
         \n\
         # Tests\n\
         - If the review flags missing tests, add minimal tests for the \
         identified gaps.\n\
         - Do not break existing tests.\n\
         \n\
         # Iteration Feedback\n\
         - If PRIOR JUDGE FEEDBACK is provided below, address every listed item.\n\
         \n\
         # Context7 — Documentation Lookup\n\
         - Before applying fixes, use the Context7 MCP tool to look up the \
         latest documentation for any libraries or APIs involved in the fix.\n\
         - Always call `resolve-library-id` first to get the Context7-compatible \
         library ID, then call `get-library-docs` to fetch the documentation.\n\
         - Use the retrieved documentation to ensure fixes use correct and \
         current API patterns.\n\
         \n\
         # Response Constraints\n\
         - Produce only the file changes. Keep responses focused and under \
         4000 tokens where possible.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

pub fn build_fixer_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plan: Option<&str>,
    selected_skills_section: Option<&str>,
    review_output: &str,
    judge_feedback: Option<&str>,
    handoff_json: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("--- Original Prompt ---\n{original_prompt}"),
        format!("--- Enhanced Prompt ---\n{enhanced_prompt}"),
    ];
    if let Some(p) = plan {
        parts.push(format!("--- Approved Plan ---\n{p}"));
    }
    if let Some(skills) = selected_skills_section {
        if !skills.trim().is_empty() {
            parts.push(skills.to_string());
        }
    }
    parts.push(format!("--- Review Output ---\n{review_output}"));
    if let Some(feedback) = judge_feedback {
        parts.push(feedback.to_string());
    }
    if let Some(handoff) = handoff_json {
        parts.push(format!("ITERATION HANDOFF:\n{handoff}"));
    }
    parts.push(
        "Inspect repository changes using tools before editing.\n\
         Do not assume a diff is provided in the prompt.\n\
         Apply concrete fixes."
            .to_string(),
    );
    parts.join("\n\n")
}
