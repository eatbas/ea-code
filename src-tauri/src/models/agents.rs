use serde::{Deserialize, Serialize};

/// Agent role identifiers for the orchestration pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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
    #[serde(alias = "copilot")]
    Codex,
    Gemini,
    Kimi,
    OpenCode,
}

pub(crate) fn default_executive_summary_model() -> String {
    "codex-5.3".to_string()
}

pub(crate) fn default_kimi_path() -> String {
    "kimi".to_string()
}

pub(crate) fn default_opencode_path() -> String {
    "opencode".to_string()
}

pub(crate) fn default_copilot_path() -> String {
    "gh".to_string()
}

pub(crate) fn default_kimi_model() -> String {
    "kimi-code".to_string()
}

pub(crate) fn default_opencode_model() -> String {
    "opencode/glm-5".to_string()
}

pub(crate) fn default_copilot_model() -> String {
    "default".to_string()
}
