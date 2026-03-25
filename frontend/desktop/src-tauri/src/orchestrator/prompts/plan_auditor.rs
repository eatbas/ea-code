//! Prompt builders for the Plan Auditor stage.

use super::PromptMeta;

pub fn build_plan_auditor_system(meta: &PromptMeta, output_path: Option<&str>) -> String {
    let output_instruction = match output_path {
        Some(path) => format!(
            "- Write your improved plan to this file (relative to workspace root): {path}\n\
             That is the ONLY file you may create or write to."
        ),
        None => "- If an OUTPUT FILE path is provided at the end of the prompt, write \
         your improved plan there. That is the ONLY file you may write."
            .to_string(),
    };

    format!(
        "# Role\n\
         You are the Plan Improver agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Your ONLY job is to review the planner output and produce an \
         improved plan text. A separate Coder agent will implement it.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - NEVER write code into source files. You are NOT the Coder.\n\
         - NEVER execute shell commands that change the file system.\n\
         - You may use read-only tools (Read, Grep, Glob, List) to verify \
         the plan against the codebase.\n\
         {output_instruction}\n\
         \n\
         # Improvement Requirements\n\
         - Make the plan stronger while keeping the original intent unchanged.\n\
         - Remove ambiguity and risky assumptions.\n\
         - Ensure steps are implementable by coding agents.\n\
         - If the planner draft is weak, rewrite it into a stronger plan.\n\
         \n\
         # Output Constraints\n\
         - Return ONLY the improved plan text as your response.\n\
         - No verdict lines, no reasoning section — just the plan itself.\n\
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
        parts.push(feedback.to_string());
    }
    parts.join("\n\n")
}

/// System prompt for the Plan Merger when merging multiple parallel plans.
pub fn build_plan_auditor_merge_system(meta: &PromptMeta, output_path: Option<&str>) -> String {
    let output_instruction = match output_path {
        Some(path) => format!(
            "- Write your merged plan to this file (relative to workspace root): {path}\n\
             That is the ONLY file you may create or write to."
        ),
        None => "- If an OUTPUT FILE path is provided at the end of the prompt, write \
         your merged plan there. That is the ONLY file you may write."
            .to_string(),
    };

    format!(
        "# Role\n\
         You are the Plan Merger agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         You receive multiple independent plans from parallel planners. Your ONLY \
         job is to merge them into ONE unified, improved plan. \
         A separate Coder agent will implement it.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - NEVER write code into source files. You are NOT the Coder.\n\
         - NEVER execute shell commands that change the file system.\n\
         - You may use read-only tools (Read, Grep, Glob, List) to verify \
         the plans against the codebase.\n\
         {output_instruction}\n\
         \n\
         # Merging Strategy\n\
         1. Identify steps that MULTIPLE planners agree on — high-confidence.\n\
         2. Where planners diverge, pick the strongest approach.\n\
         3. Remove duplicates and consolidate overlapping steps.\n\
         4. Ensure the final plan is complete, ordered, and implementation-ready.\n\
         5. Make the merged plan stronger — remove ambiguity and risky assumptions.\n\
         \n\
         # Output Constraints\n\
         - Return ONLY the merged plan text as your response.\n\
         - No verdict lines, no reasoning section — just the plan itself.\n\
         - No markdown fences.",
        iter = meta.iteration,
        max = meta.max_iterations,
    )
}

/// User message for the Plan Merger when merging multiple parallel plans.
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
        parts.push(feedback.to_string());
    }
    parts.join("\n\n")
}
