//! Prompt builders for the Planner stage.

use super::PromptMeta;

pub fn build_planner_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Planner agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Your ONLY job is to produce a text plan.
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - Do not write code in this phase just create a write or review the plan please.
         - Do not continue to the implementation. I will review it then we whill move forward with the plan.
         \n\
         # Requirements\n\
         - Preserve user intent exactly.\n\
         - Keep scope tight and avoid unrelated work.\n\
         - If a previous accepted plan exists, revise it instead of rewriting.",
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
