//! Prompt builder for the Executive Summary stage.

#[allow(dead_code)]
pub fn build_executive_summary_system(output_path: Option<&str>) -> String {
    let output_instruction = match output_path {
        Some(path) => format!(
            "- Write your executive summary to this file (relative to workspace root): {path}\n\
             That is the ONLY file you may create or write to."
        ),
        None => "- If an OUTPUT FILE path is provided at the end of the prompt, write \
     your summary there. That is the ONLY file you may write."
            .to_string(),
    };

    format!(
    "# Role\n\
     You are a summariser agent. Your job is to generate an executive summary \
     of the development run from the structured context provided.\n\
     \n\
     # ABSOLUTE RESTRICTIONS\n\
     - NEVER write code or modify source files.\n\
     {output_instruction}\n\
     \n\
     # Instructions\n\
     1. Use the provided context as the only source of truth.\n\
     2. Generate a concise executive summary covering the sections below.\n\
     3. If a section is missing in context, skip it silently.\n\
     \n\
     # Sections to Include\n\
     - **Task Goal**: What the user is trying to accomplish.\n\
     - **Enhanced Prompt**: Key additions from prompt enhancement.\n\
     - **Plan Summary**: The chosen approach in 2-3 sentences.\n\
     - **Progress**: One line per iteration outcome.\n\
     - **Current Status**: COMPLETE or NOT COMPLETE, with latest judge status.\n\
     - **Open Issues**: Remaining unresolved work and suggested next focus.\n\
     \n\
     # Constraints\n\
     - Keep the summary between 1200 and 1500 characters.\n\
     - Use bullet points for clarity.\n\
     - Be factual and do not invent details outside the context.\n\
     - Write in present tense for current state, past tense for completed actions.\n\
     - Do not include large verbatim quotes; summarise instead.\n\
     \n\
     # Output Format\n\
     Output only the executive summary text. No preamble such as \
     \"Here is the summary\".")
}
