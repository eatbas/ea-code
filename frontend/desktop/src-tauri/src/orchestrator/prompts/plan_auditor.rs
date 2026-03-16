//! Prompt builders for the Plan Auditor stage.

use super::PromptMeta;

pub fn build_plan_auditor_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Plan Auditor agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Audit and improve the Planner output for correctness, completeness, \
         and feasibility.\n\
         \n\
         # CRITICAL: Auditing Only — No Code Changes\n\
         - DO NOT create, modify, edit, or delete any files.\n\
         - DO NOT run any commands that change the file system.\n\
         - DO NOT write any code or make any changes to the codebase.\n\
         - You may READ files to verify the plan, but NEVER write to them.\n\
         - Your ONLY job is to OUTPUT the audited plan text. A separate Coder \
         agent will implement it later.\n\
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
         - No markdown fences.",
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
    judge_feedback: Option<&str>,
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
    if let Some(feedback) = judge_feedback {
        parts.push(format!(
            "--- Judge Feedback From Previous Iteration ---\n{feedback}"
        ));
    }
    parts.join("\n\n")
}

/// System prompt for the Plan Auditor when merging 2-3 parallel plans.
pub fn build_plan_auditor_merge_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Plan Merger & Auditor agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         You receive multiple independent plans from parallel planners. Your job is \
         to synthesise them into ONE unified plan, then audit it.\n\
         \n\
         # CRITICAL: Merging & Auditing Only — No Code Changes\n\
         - DO NOT create, modify, edit, or delete any files.\n\
         - DO NOT run any commands that change the file system.\n\
         - You may READ files to verify the plans, but NEVER write to them.\n\
         - Your ONLY job is to OUTPUT the merged and audited plan text.\n\
         \n\
         # Merging Strategy\n\
         1. Identify steps that MULTIPLE planners agree on — these are high-confidence \
         and should be included.\n\
         2. Where planners DIVERGE, evaluate the reasoning and pick the strongest \
         approach. Note disagreements briefly.\n\
         3. Remove duplicates and consolidate overlapping steps.\n\
         4. Ensure the final plan is complete, ordered, and implementation-ready.\n\
         \n\
         # Audit Requirements\n\
         - Keep original intent unchanged.\n\
         - Remove ambiguity and risky assumptions.\n\
         - Ensure steps are implementable by coding agents.\n\
         - The first line MUST be exactly APPROVED or REJECTED.\n\
         - Use this exact section header: --- Improved Plan ---\n\
         \n\
         # Output Constraints\n\
         - Return only the merged, audited final plan text.\n\
         - No markdown fences.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

/// User message for the Plan Auditor when merging 2-3 parallel plans.
pub fn build_plan_auditor_merge_user(
    original_prompt: &str,
    enhanced_prompt: &str,
    plans: &[(String, String)],
    previous_plan: Option<&str>,
    judge_feedback: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("--- Original Prompt ---\n{original_prompt}"),
        format!("--- Enhanced Prompt ---\n{enhanced_prompt}"),
    ];
    for (label, plan_text) in plans {
        parts.push(format!("--- {label} ---\n{plan_text}"));
    }
    if let Some(prev) = previous_plan {
        parts.push(format!("--- Previous Accepted Plan ---\n{prev}"));
    }
    if let Some(feedback) = judge_feedback {
        parts.push(format!(
            "--- Judge Feedback From Previous Iteration ---\n{feedback}"
        ));
    }
    parts.join("\n\n")
}
