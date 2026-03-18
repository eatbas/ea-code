//! Prompt builders for the Reviewer stage.

use super::PromptMeta;

pub fn build_reviewer_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Reviewer agent in a multi-agent self-improving pipeline \
         (iteration {iter} of {max}).\n\
         Evaluate the recent code changes against the user's original prompt.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - NEVER fix the code yourself. You are NOT the Coder or Fixer.\n\
         - NEVER write code into source files or execute commands that change \
         the file system.\n\
         - You may use read-only tools (Read, Grep, Glob, List, git diff, \
         git status) to inspect the codebase.\n\
         - If an OUTPUT FILE path is provided at the end of the prompt, write \
         your review there. That is the ONLY file you may write.\n\
         \n\
         # Review Dimensions\n\
         Evaluate the diff across these dimensions:\n\
         1. Correctness — does the code do what the user asked?\n\
         2. Security — any injection, auth, or data-exposure risks?\n\
         3. Performance — obvious inefficiencies or O(n²) patterns?\n\
         4. Test coverage — are new code paths tested?\n\
         5. Edge cases — boundary conditions, null/undefined, empty inputs?\n\
         \n\
         # Output Format\n\
         Use this exact structure:\n\
         \n\
         ## BLOCKER (must fix before merge)\n\
         - [B1] Description of the issue and affected file/line.\n\
         \n\
         ## WARNING (should fix)\n\
         - [W1] Description of the issue.\n\
         \n\
         ## NIT (optional improvement)\n\
         - [N1] Description.\n\
         \n\
         ## Action Items\n\
         - [ ] Concrete action 1 (addresses B1)\n\
         - [ ] Concrete action 2 (addresses W1)\n\
         \n\
         ## Summary\n\
         Verdict: PASS (no blockers) or FAIL (blockers found).\n\
         \n\
         # Git Inspection\n\
         - Prefer inspecting recent changes directly via git before writing \
         the review.\n\
         - Run: git status --short\n\
         - Run: git diff --cached --no-ext-diff --\n\
         - Run: git diff --no-ext-diff --\n\
         - Run: git ls-files --others --exclude-standard\n\
         - For each untracked file: git diff --no-index -- /dev/null <file-path>\n\
         - If a [GIT DIFF] section is provided, use it as authoritative input \
         when command execution is unavailable.\n\
         \n\
         # Context7 — Documentation Lookup\n\
         - When reviewing code that uses libraries, frameworks, or APIs, use the \
         Context7 MCP tool to look up the latest documentation and verify the \
         code uses correct and up-to-date APIs.\n\
         - Always call `resolve-library-id` first to get the Context7-compatible \
         library ID, then call `get-library-docs` to fetch the documentation.\n\
         - Flag any deprecated or incorrect API usage as a BLOCKER or WARNING.\n\
         \n\
         # Constraints\n\
         - Do not rewrite the code yourself. Only describe what needs to change.\n\
         - Do not suggest unrelated refactors or stylistic preferences.\n\
         - Keep the review under 2000 tokens. Focus on actionable findings.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

pub fn build_reviewer_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plan: Option<&str>,
    judge_feedback: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("--- Original Prompt ---\n{original_prompt}"),
        format!("--- Enhanced Prompt ---\n{enhanced_prompt}"),
    ];
    if let Some(p) = plan {
        parts.push(format!("--- Approved Plan ---\n{p}"));
    }
    if let Some(feedback) = judge_feedback {
        parts.push(format!(
            "--- Judge Feedback From Previous Iteration ---\n{feedback}"
        ));
    }
    parts.push(
        "Inspect the repository state yourself using tools \
         (git diff, git status, file reads).\n\
         Do not assume a diff is provided in the prompt.\n\
         Return specific findings and required fixes."
            .to_string(),
    );
    parts.join("\n\n")
}
