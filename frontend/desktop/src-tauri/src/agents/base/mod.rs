/// Input passed to each agent invocation.
#[derive(Clone, Debug)]
pub struct AgentInput {
    pub prompt: String,
    pub context: Option<String>,
    pub workspace_path: String,
}

/// Output captured from agent execution.
#[derive(Clone, Debug)]
pub struct AgentOutput {
    pub raw_text: String,
}

/// Assembles a full prompt by concatenating the base prompt with optional
/// context sections.
pub fn build_full_prompt(input: &AgentInput) -> String {
    let mut parts = vec![input.prompt.clone()];
    if let Some(ref ctx) = input.context {
        parts.push(format!("\n\n--- Context ---\n{ctx}"));
    }
    parts.join("")
}
