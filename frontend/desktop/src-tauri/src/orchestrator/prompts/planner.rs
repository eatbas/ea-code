//! Prompt builders for the Planner stage.

use super::PromptMeta;

pub fn build_planner_system(meta: &PromptMeta, output_path: Option<&str>) -> String {
    let output_section = match output_path {
        Some(path) => format!(
            "# Output Format\n\
             - Write your plan to this file (relative to workspace root): {path}\n\
             That is the ONLY file you may create or write to.\n\
             - Write the plan content directly — do NOT use --- BEGIN PLAN --- / --- END PLAN --- markers.\n\
             - The plan must be a clear, numbered list of implementation steps.\n\
             - Do NOT include internal reasoning, tool output, or code in the plan."
        ),
        None => "# Output Format\n\
         - When your plan is ready, wrap it in these exact markers on their own lines:\n\
         \n\
           --- BEGIN PLAN ---\n\
           (your plan here)\n\
           --- END PLAN ---\n\
         \n\
         - Everything between these markers is the plan. Everything outside is discarded.\n\
         - Do NOT include internal reasoning, tool output, or code between the markers.\n\
         - The plan must be a clear, numbered list of implementation steps."
            .to_string(),
    };

    format!(
        "# Role\n\
         You are the Planner agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Your ONLY job is to produce a text plan. A separate Coder agent will implement it.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - Do NOT write code into source files. You are NOT the Coder.\n\
         - Do NOT execute shell commands that change the file system.\n\
         - You may use read-only tools (Read, Grep, Glob, List) to understand the codebase.\n\
         \n\
         # Requirements\n\
         - Preserve user intent exactly.\n\
         - Keep scope tight and avoid unrelated work.\n\
         - If a previous accepted plan exists, revise it instead of rewriting.\n\
         \n\
         {output_section}",
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
        parts.push(feedback.to_string());
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_user_includes_judge_feedback() {
        let user = build_planner_user("task", "enhanced", None, Some("Fix blockers"));
        assert!(user.contains("Fix blockers"));
        assert!(user.contains("Fix blockers"));
    }
}
