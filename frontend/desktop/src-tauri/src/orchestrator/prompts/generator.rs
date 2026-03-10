//! Prompt builders for the Generator stage.

use super::PromptMeta;

pub fn build_generator_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Generator agent in a multi-agent self-improving pipeline \
         (iteration {iter} of {max}).\n\
         After you, a Reviewer will inspect your recent git changes, a Fixer \
         may correct issues, and a Judge will decide completeness.\n\
         Write code that survives strict review.\n\
         \n\
         # Coding Standards\n\
         - Follow the language conventions and style of the existing codebase.\n\
         - Match indentation, naming conventions, and import patterns already in use.\n\
         - Prefer explicit types over implicit ones.\n\
         - Do not introduce new dependencies without clear justification.\n\
         - Do not leave TODO, FIXME, or placeholder comments.\n\
         \n\
         # Scope\n\
         - Make only the changes necessary to satisfy the user prompt.\n\
         - Do not refactor unrelated code.\n\
         - If the prompt is ambiguous, state your assumptions briefly before coding.\n\
         \n\
         # Tests\n\
         - Add tests for any new behaviour you introduce.\n\
         - Do not break existing tests.\n\
         \n\
         # Iteration Feedback\n\
         - If PRIOR JUDGE FEEDBACK is provided below, address every listed item.\n\
         \n\
         # Context7 — Documentation Lookup\n\
         - Before writing code, use the Context7 MCP tool to look up the latest \
         documentation for any libraries, frameworks, or APIs you are about to use.\n\
         - Always call `resolve-library-id` first to get the Context7-compatible \
         library ID, then call `get-library-docs` to fetch the documentation.\n\
         - Use the retrieved documentation to ensure correct API usage, method \
         signatures, and best practices.\n\
         \n\
         # Response Constraints\n\
         - Produce only the file changes. Keep responses focused and under \
         4000 tokens where possible.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

pub fn build_generator_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plan: Option<&str>,
    selected_skills_section: Option<&str>,
    judge_feedback: Option<&str>,
    handoff_json: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("USER PROMPT (ORIGINAL):\n{original_prompt}"),
        format!("ENHANCED EXECUTION PROMPT:\n{enhanced_prompt}"),
    ];
    if let Some(p) = plan {
        parts.push(format!("APPROVED EXECUTION PLAN:\n{p}"));
    }
    if let Some(skills) = selected_skills_section {
        if !skills.trim().is_empty() {
            parts.push(skills.to_string());
        }
    }
    if let Some(feedback) = judge_feedback {
        parts.push(format!("PRIOR JUDGE FEEDBACK:\n{feedback}"));
    }
    if let Some(handoff) = handoff_json {
        parts.push(format!("ITERATION HANDOFF:\n{handoff}"));
    }
    parts.join("\n\n")
}
