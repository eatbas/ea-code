use crate::models::PipelineAgent;

pub fn agent_label(agent: &PipelineAgent) -> String {
    format!("{} / {}", agent.provider, agent.model)
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
