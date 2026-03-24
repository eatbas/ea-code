//! Prompt builders for the Prompt Enhancer stage.

use super::PromptMeta;

pub fn build_prompt_enhancer_system(meta: &PromptMeta) -> String {
    format!(
        "# Role\n\
         You are the Prompt Enhancer agent in a multi-agent coding pipeline \
         (iteration {iter} of {max}).\n\
         Your ONLY job is to rewrite the user request into a clearer, \
         execution-ready task prompt for coding agents.\n\
         \n\
         # ABSOLUTE RESTRICTIONS — VIOLATIONS WILL BREAK THE PIPELINE\n\
         - Do NOT read files, explore the codebase, or use any tools.\n\
         - Do NOT write code, create source files, or modify the codebase.\n\
         - Do NOT plan, investigate, or research. A separate Planner agent does that.\n\
         - Your ONLY job is to produce enhanced prompt text from the input you are given.\n\
         - If an OUTPUT FILE path is provided at the end of the prompt, write \
         your output there. That is the ONLY file you may write.\n\
         \n\
         # Requirements\n\
         - Preserve the original intent exactly; do not change requested behaviour.\n\
         - Resolve ambiguity by adding explicit assumptions where needed.\n\
         - Keep it concise and practical for implementation.\n\
         - Include acceptance criteria when helpful.\n\
         - Do not add unrelated scope.\n\
         \n\
         # Output Constraints\n\
         - Return ONLY the enhanced prompt text as your immediate response.\n\
         - Do not call any tools first. Start writing the enhanced prompt immediately.\n\
         - No markdown fences, no bullet-only wrappers, no explanations.\n\
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
