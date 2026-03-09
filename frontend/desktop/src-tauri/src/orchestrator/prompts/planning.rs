//! Prompt builders for pre-execution stages: enhancer, planner, plan auditor.

use super::PromptMeta;

// ---------------------------------------------------------------------------
// Prompt Enhancer
// ---------------------------------------------------------------------------

pub fn build_prompt_enhancer_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Prompt Enhancer agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Your job is to rewrite the user request into a clearer, \
         execution-ready task prompt for coding agents.\n\
         \n\
         # Requirements\n\
         - Preserve the original intent exactly; do not change requested behaviour.\n\
         - Resolve ambiguity by adding explicit assumptions where needed.\n\
         - Keep it concise and practical for implementation.\n\
         - Include acceptance criteria when helpful.\n\
         - Do not add unrelated scope.\n\
         \n\
         # Context7 — Documentation Lookup\n\
         - Before rewriting the prompt, use the Context7 MCP tool to look up \
         the latest documentation for any libraries, frameworks, or APIs \
         mentioned in the user request.\n\
         - Always call `resolve-library-id` first to get the Context7-compatible \
         library ID, then call `get-library-docs` to fetch the documentation.\n\
         - Incorporate relevant API details, correct method signatures, and \
         version-specific information into the enhanced prompt.\n\
         \n\
         # Output Constraints\n\
         - Return only the enhanced prompt text.\n\
         - No markdown fences, no bullet-only wrappers, no explanations before/after.\n\
         - Keep output under 1200 tokens.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

pub fn build_prompt_enhancer_user(user_prompt: &str) -> String {
    format!("ORIGINAL USER PROMPT:\n{user_prompt}")
}

// ---------------------------------------------------------------------------
// Planner
// ---------------------------------------------------------------------------

pub fn build_planner_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Planner agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Create a practical, execution-ready implementation plan for coding agents.\n\
         \n\
         # Requirements\n\
         - Preserve user intent exactly.\n\
         - Keep scope tight and avoid unrelated work.\n\
         - Produce concrete steps with clear order.\n\
         - Include validation and test expectations where relevant.\n\
         \n\
         # Inputs\n\
         - You may receive the original prompt, enhanced prompt, previous \
         accepted plan, and user revision feedback.\n\
         - If previous accepted plan exists, revise it instead of rewriting \
         from scratch.\n\
         \n\
         # Output Constraints\n\
         - Return only the plan text.\n\
         - No markdown fences.\n\
         - Keep output under 1500 tokens.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

pub fn build_planner_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    previous_plan: Option<&str>,
    judge_feedback: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("USER PROMPT (ORIGINAL):\n{original_prompt}"),
        format!("ENHANCED EXECUTION PROMPT:\n{enhanced_prompt}"),
    ];
    if let Some(plan) = previous_plan {
        parts.push(format!("PREVIOUS ACCEPTED PLAN:\n{plan}"));
    }
    if let Some(feedback) = judge_feedback {
        parts.push(format!(
            "JUDGE FEEDBACK FROM PREVIOUS ITERATION:\n{feedback}"
        ));
    }
    parts.join("\n\n")
}

// ---------------------------------------------------------------------------
// Plan Auditor
// ---------------------------------------------------------------------------

pub fn build_plan_auditor_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Plan Auditor agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Audit and improve the Planner output for correctness, completeness, \
         and feasibility.\n\
         \n\
         # Requirements\n\
         - Keep original intent unchanged.\n\
         - Remove ambiguity and risky assumptions.\n\
         - Ensure steps are implementable by coding agents.\n\
         - Keep plan concise and ordered.\n\
         - The first line MUST be exactly APPROVED or REJECTED.\n\
         - Then improve and rewrite the plan so it is implementation-ready.\n\
         - Use this exact section header before the rewritten plan: \
         --- Improved Plan ---\n\
         \n\
         # Inputs\n\
         - You may receive original prompt, enhanced prompt, planner draft, \
         previous accepted plan, and latest user feedback.\n\
         - If planner draft is weak, rewrite it into a stronger final plan.\n\
         \n\
         # Output Constraints\n\
         - Return only the audited final plan text.\n\
         - No markdown fences.\n\
         - Keep output under 1500 tokens.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

pub fn build_plan_auditor_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plan_draft: &str,
    previous_plan: Option<&str>,
    user_feedback: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("--- Original Prompt ---\n{original_prompt}"),
        format!("--- Enhanced Prompt ---\n{enhanced_prompt}"),
        format!("--- Proposed Plan ---\n{plan_draft}"),
    ];
    if let Some(prev) = previous_plan {
        parts.push(format!("--- Previous Accepted Plan ---\n{prev}"));
    }
    if let Some(fb) = user_feedback {
        parts.push(format!("--- Latest User Feedback ---\n{fb}"));
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_enhancer_includes_iteration() {
        let meta = PromptMeta {
            iteration: 2,
            max_iterations: 3,
            previous_judge_output: None,
        };
        let system = build_prompt_enhancer_system(&meta);
        assert!(system.contains("iteration 2 of 3"));
    }

    #[test]
    fn planner_user_includes_judge_feedback() {
        let user = build_planner_user("task", "enhanced", None, Some("Fix blockers"));
        assert!(user.contains("JUDGE FEEDBACK FROM PREVIOUS ITERATION"));
        assert!(user.contains("Fix blockers"));
    }
}
