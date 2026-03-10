//! Prompt builders for the Prompt Enhancer stage.

use super::PromptMeta;

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
         - This stage is rewrite-only: do not run commands, do not call tools, \
         do not edit files, and do not claim code was implemented.\n\
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
}
