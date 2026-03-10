//! Prompt builders for the Planner stage.

use super::PromptMeta;

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
         - No markdown fences.",
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
