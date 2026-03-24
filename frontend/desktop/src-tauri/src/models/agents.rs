use serde::{Deserialize, Serialize};

/// Agent role identifiers for the orchestration pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AgentRole {
    PromptEnhancer,
    Planner,
    PlanAuditor,
    Coder,
    ReviewerAuditor,
    CodeFixer,
    Judge,
    ExecutiveSummary,
}

/// Supported CLI agent backends.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentBackend {
    Claude,
    Codex,
    Gemini,
    Kimi,
    OpenCode,
    Copilot,
}

pub(crate) fn default_executive_summary_model() -> String {
    "gpt-5.3-codex".to_string()
}

pub(crate) fn default_kimi_path() -> String {
    "kimi".to_string()
}

pub(crate) fn default_opencode_path() -> String {
    "opencode".to_string()
}

pub(crate) fn default_kimi_model() -> String {
    "kimi-code/kimi-for-coding".to_string()
}

pub(crate) fn default_opencode_model() -> String {
    "opencode/glm-5".to_string()
}
