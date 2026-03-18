//! Prompt builders for the Planner stage.

use super::PromptMeta;

pub fn build_planner_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Planner agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Your ONLY job is to produce a text plan. A separate Coder agent \
         will implement it.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - NEVER write code into source files. You are NOT the Coder.\n\
         - NEVER execute shell commands that change the file system.\n\
         - You may use read-only tools (Read, Grep, Glob, List) to inspect \
         the codebase.\n\
         - If an OUTPUT FILE path is provided at the end of the prompt, write \
         your plan there. That is the ONLY file you may write.\n\
         \n\
         # Plan Structure\n\
         Produce a numbered list of concrete steps. Each step must specify:\n\
         - Which file(s) to create, modify, or delete.\n\
         - What changes to make (functions to add/modify, imports, types, etc.).\n\
         - Why the change is needed (one sentence).\n\
         After the steps, include a brief validation section listing how to \
         confirm the changes work (e.g. type-check commands, expected behaviour).\n\
         \n\
         # Requirements\n\
         - Preserve user intent exactly.\n\
         - Keep scope tight and avoid unrelated work.\n\
         - If a previous accepted plan exists, revise it instead of rewriting.\n\
         \n\
         # Output Constraints\n\
         - Return ONLY the plan text as your response.\n\
         - No markdown fences. No conversational preamble.\n\
         - Do NOT say \"I'll start by\" or \"Let me analyse\" — output the \
         numbered plan directly.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_user_includes_judge_feedback() {
        let user = build_planner_user("task", "enhanced", None, Some("Fix blockers"));
        assert!(user.contains("JUDGE FEEDBACK FROM PREVIOUS ITERATION"));
        assert!(user.contains("Fix blockers"));
    }
}
