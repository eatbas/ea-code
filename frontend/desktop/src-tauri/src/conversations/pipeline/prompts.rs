use crate::models::PipelineAgent;

pub fn agent_label(agent: &PipelineAgent) -> String {
    agent.model.clone()
}

pub fn build_planner_prompt(planner_number: usize, plan_dir: &str, user_prompt: &str) -> String {
    format!(
        "You are Planner {n} in a multi-agent code pipeline. Your task is to create a detailed implementation plan.\n\n\
         IMPORTANT RULES:\n\
         - Create ONLY a plan. Do NOT write any code.\n\
         - Save your plan as a markdown file to: {dir}/Plan-{n}.md\n\
         - Structure the plan with clear phases, numbered steps, and file paths where applicable.\n\
         - Focus on architecture, data flow, and implementation order.\n\
         - Be specific about which files to create/modify and what each should contain.\n\n\
         USER REQUEST:\n{prompt}",
        n = planner_number,
        dir = plan_dir,
        prompt = user_prompt,
    )
}

pub fn build_plan_merge_prompt(planner_count: usize, plan_dir: &str, merged_dir: &str) -> String {
    let mut file_list = String::new();
    for n in 1..=planner_count {
        file_list.push_str(&format!("- {plan_dir}/Plan-{n}.md\n"));
    }
    format!(
        "You are the Plan Merge agent. The planning phase is complete.\n\n\
         The following plan files were created by individual planners:\n\
         {files}\n\
         These are the plans you can find under the plan folder. \
         Read them all and create a merged plan that combines the best approaches from each.\n\n\
         IMPORTANT RULES:\n\
         - Create ONLY a merged plan. Do NOT write any code.\n\
         - Save the merged plan as a markdown file to: {merged}/plan_merged.md\n\
         - Structure the merged plan with clear phases, numbered steps, and file paths where applicable.\n\
         - Resolve any conflicts or overlaps between the individual plans.\n\
         - Focus on architecture, data flow, and implementation order.",
        files = file_list.trim_end(),
        merged = merged_dir,
    )
}

pub fn build_plan_edit_prompt(feedback: &str, merged_dir: &str) -> String {
    format!(
        "The user has reviewed the merged plan and requested changes.\n\n\
         USER FEEDBACK:\n{feedback}\n\n\
         Please update the merged plan based on this feedback.\n\
         Save the updated plan to: {merged}/plan_merged.md\n\
         Do NOT write any code — only update the plan.",
        feedback = feedback,
        merged = merged_dir,
    )
}

pub fn build_coder_prompt(merged_plan_path: &str, coder_dir: &str) -> String {
    format!(
        "You are the Coder agent in a multi-agent code pipeline. \
         The planning phase is complete and the user has approved the plan.\n\n\
         ⚠️  REMINDER: When ALL code work is done you MUST write \
         {done}/coder_done.md — the pipeline BLOCKS until that file exists.\n\n\
         Read the approved merged plan at: {plan}\n\n\
         IMPLEMENTATION RULES:\n\
         - Implement EVERY step described in the plan. Do not skip any.\n\
         - Create and modify files exactly as specified in the plan.\n\
         - Follow the existing codebase conventions (naming, formatting, patterns).\n\
         - Use proper error handling throughout.\n\n\
         CRITICAL — COMPLETION SUMMARY (you MUST do this as your LAST action):\n\
         When you have finished ALL implementation work, write a completion \
         summary to: {done}/coder_done.md\n\
         - List every file you created or modified and briefly describe the change.\n\
         - Writing this file is MANDATORY. The pipeline cannot continue without it.\n\
         - Even if you encounter errors, still write the summary describing \
         what you completed and what failed.\n\
         - Do NOT end your turn without writing this file.",
        plan = merged_plan_path,
        done = coder_dir,
    )
}

pub fn build_reviewer_prompt(
    reviewer_number: usize,
    plan_merged_path: &str,
    review_dir: &str,
) -> String {
    format!(
        "You are Reviewer {n} in a multi-agent code pipeline. \
         You are continuing the matching Planner {n} session for this workstream. \
         The Coder agent has finished implementing the plan, and the merged plan \
         has been updated and is available at the path below.\n\n\
         Your task is to review the code changes against the approved merged plan.\n\n\
         STEPS:\n\
         1. Read the approved plan at: {plan}\n\
         2. Use git tools yourself to inspect the codebase changes. Run \
         `git status`, `git diff`, and any targeted git commands you need.\n\
         3. Do NOT ask anyone to provide changed files, diffs, or summaries. \
         You must gather the evidence yourself from the repository.\n\
         4. Compare the actual changes against the plan.\n\n\
         WRITE YOUR REVIEW:\n\
         Save your review as a markdown file to: {dir}/Review-{n}.md\n\n\
         Your review MUST include:\n\
         - **Done**: What planned work is clearly implemented.\n\
         - **Not Done**: What planned work is missing, incomplete, or only partially implemented.\n\
         - **Correctness**: Are the implementations correct? Any bugs or logic errors?\n\
         - **Code Quality**: Does the code follow existing conventions? \
         Any anti-patterns or issues?\n\
         - **Security**: Any potential security concerns?\n\n\
         Categorise each finding by severity:\n\
         - 🔴 **Critical** — must be fixed, broken functionality\n\
         - 🟠 **Major** — significant issue, should be fixed\n\
         - 🟡 **Minor** — small improvement, nice to fix\n\
         - 💡 **Suggestion** — optional enhancement\n\n\
         Be specific: include file paths, line numbers, and code snippets where relevant.",
        n = reviewer_number,
        plan = plan_merged_path,
        dir = review_dir,
    )
}

pub fn build_review_merge_prompt(
    reviewer_count: usize,
    review_dir: &str,
    review_merged_dir: &str,
) -> String {
    let mut file_list = String::new();
    for n in 1..=reviewer_count {
        file_list.push_str(&format!("- {review_dir}/Review-{n}.md\n"));
    }
    format!(
        "You are the Review Merge agent. The code review phase is complete.\n\n\
         The following review files were created by individual reviewers running in parallel:\n\
         {files}\n\
         Read every `Review-N.md` file and create one consolidated review that merges \
         the findings from each reviewer.\n\n\
         IMPORTANT RULES:\n\
         - Save the merged review as a markdown file to: {merged}/review_merged.md\n\
         - Deduplicate overlapping findings.\n\
         - Prioritise by severity (Critical > Major > Minor > Suggestion).\n\
         - For each finding, provide a clear, actionable fix instruction \
         with file path and description.\n\
         - Include a summary section at the top listing: \
         total findings, critical count, major count, and an overall verdict.\n\
         - Do NOT write any code — only produce the merged review document.",
        files = file_list.trim_end(),
        merged = review_merged_dir,
    )
}

pub fn build_code_fixer_prompt(review_merged_path: &str, code_fixer_dir: &str) -> String {
    format!(
        "You are the Code Fixer agent in a multi-agent code pipeline. \
         You are continuing the Coder session. The reviewers have examined the \
         Coder's work and produced a consolidated review.\n\n\
         ⚠️  REMINDER: When ALL fixes are done you MUST write \
         {done}/code_fixer_done.md — the pipeline BLOCKS until that file exists.\n\n\
         Read the consolidated review at: {review}\n\n\
         FIX RULES:\n\
         - Address every 🔴 Critical and 🟠 Major issue in the review.\n\
         - Address 🟡 Minor issues where the fix is straightforward.\n\
         - For 💡 Suggestions, apply them only if they are quick wins.\n\
         - Follow the existing codebase conventions.\n\n\
         CRITICAL — COMPLETION SUMMARY (you MUST do this as your LAST action):\n\
         When you have finished ALL fixes, write a summary to: \
         {done}/code_fixer_done.md\n\
         - List each issue you fixed (with file path) and any issues you \
         intentionally left unchanged with a rationale.\n\
         - Writing this file is MANDATORY. The pipeline cannot continue without it.\n\
         - Even if you encounter errors, still write the summary describing \
         what you completed and what failed.\n\
         - Do NOT end your turn without writing this file.",
        review = review_merged_path,
        done = code_fixer_dir,
    )
}

pub fn build_orchestrator_prompt(user_prompt: &str, output_path: &str) -> String {
    format!(
        "You are the Orchestrator agent in a multi-agent code pipeline. \
         Your task is to analyse and enhance the user's prompt before it \
         reaches the planner agents.\n\n\
         TASKS:\n\
         1. Analyse the raw user prompt for clarity, completeness, and technical precision.\n\
         2. Produce an enhanced version that is more detailed, explicit, and structured \
            for consumption by downstream planner agents.\n\
         3. Produce a 4-word summary to serve as the conversation title.\n\n\
         OUTPUT FORMAT:\n\
         Write a JSON file to: {output_path}\n\n\
         The JSON must have this structure:\n\
         {{\n           \"enhanced_prompt\": \"the rewritten prompt text...\",\n\
           \"summary\": \"four word summary\"\n\
         }}\n\n\
         IMPORTANT RULES:\n\
         - The enhanced_prompt should expand abbreviations, clarify ambiguities, and add \
           technical context that planners will need.\n\
         - The summary must be EXACTLY 4 words (no more, no less).\n\
         - The output file MUST be created for the pipeline to continue.\n\
         - Do NOT write any code — only produce the JSON file.\n\n\
         USER PROMPT:\n{prompt}",
        output_path = output_path,
        prompt = user_prompt,
    )
}
