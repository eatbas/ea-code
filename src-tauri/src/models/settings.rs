use serde::{Deserialize, Serialize};

use super::agents::{
    default_executive_summary_agent, default_executive_summary_model, default_kimi_model,
    default_kimi_path, default_opencode_model, default_opencode_path,
    default_prompt_enhancer_agent, AgentBackend,
};

/// Application settings persisted locally.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub claude_path: String,
    pub codex_path: String,
    pub gemini_path: String,
    #[serde(default = "default_kimi_path")]
    pub kimi_path: String,
    #[serde(default = "default_opencode_path")]
    pub opencode_path: String,
    #[serde(default = "default_prompt_enhancer_agent")]
    pub prompt_enhancer_agent: AgentBackend,
    #[serde(default)]
    pub planner_agent: Option<AgentBackend>,
    #[serde(default)]
    pub plan_auditor_agent: Option<AgentBackend>,
    pub generator_agent: AgentBackend,
    pub reviewer_agent: AgentBackend,
    pub fixer_agent: AgentBackend,
    pub final_judge_agent: AgentBackend,
    pub max_iterations: u32,
    pub require_git: bool,
    /// Comma-separated list of enabled Claude models.
    pub claude_model: String,
    /// Comma-separated list of enabled Codex models.
    pub codex_model: String,
    /// Comma-separated list of enabled Gemini models.
    pub gemini_model: String,
    /// Comma-separated list of enabled Kimi models.
    #[serde(default = "default_kimi_model")]
    pub kimi_model: String,
    /// Comma-separated list of enabled OpenCode models.
    #[serde(default = "default_opencode_model")]
    pub opencode_model: String,
    /// Per-stage model selections.
    pub prompt_enhancer_model: String,
    #[serde(default)]
    pub planner_model: Option<String>,
    #[serde(default)]
    pub plan_auditor_model: Option<String>,
    pub generator_model: String,
    pub reviewer_model: String,
    pub fixer_model: String,
    pub final_judge_model: String,
    #[serde(default = "default_executive_summary_agent")]
    pub executive_summary_agent: AgentBackend,
    #[serde(default = "default_executive_summary_model")]
    pub executive_summary_model: String,
    /// Pause pipeline after planning for user approval.
    #[serde(default)]
    pub require_plan_approval: bool,
    /// Seconds before auto-approving the plan (0 = wait indefinitely).
    #[serde(default = "default_plan_timeout")]
    pub plan_auto_approve_timeout_sec: u32,
    /// Maximum user revision rounds for the plan.
    #[serde(default = "default_max_plan_revisions")]
    pub max_plan_revisions: u32,
    /// Use compact handoff mode to reduce token usage.
    #[serde(default)]
    pub token_optimized_prompts: bool,
    /// Number of retries per agent call on failure (0 = no retry).
    #[serde(default = "default_agent_retry_count")]
    pub agent_retry_count: u32,
    /// Per-agent timeout in milliseconds (0 = no timeout).
    #[serde(default)]
    pub agent_timeout_ms: u64,
}

fn default_plan_timeout() -> u32 {
    45
}

fn default_max_plan_revisions() -> u32 {
    3
}

fn default_agent_retry_count() -> u32 {
    1
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            codex_path: "codex".to_string(),
            gemini_path: "gemini".to_string(),
            kimi_path: "kimi".to_string(),
            opencode_path: "opencode".to_string(),
            prompt_enhancer_agent: AgentBackend::Claude,
            planner_agent: None,
            plan_auditor_agent: None,
            generator_agent: AgentBackend::Claude,
            reviewer_agent: AgentBackend::Codex,
            fixer_agent: AgentBackend::Claude,
            final_judge_agent: AgentBackend::Codex,
            max_iterations: 3,
            require_git: true,
            claude_model: "sonnet".to_string(),
            codex_model: "codex-5.3".to_string(),
            gemini_model: "gemini-2.5-pro".to_string(),
            kimi_model: "kimi-k2.5".to_string(),
            opencode_model: "opencode/glm-5".to_string(),
            prompt_enhancer_model: "sonnet".to_string(),
            planner_model: None,
            plan_auditor_model: None,
            generator_model: "sonnet".to_string(),
            reviewer_model: "codex-5.3".to_string(),
            fixer_model: "sonnet".to_string(),
            final_judge_model: "codex-5.3".to_string(),
            executive_summary_agent: AgentBackend::Codex,
            executive_summary_model: "codex-5.3".to_string(),
            require_plan_approval: false,
            plan_auto_approve_timeout_sec: 45,
            max_plan_revisions: 3,
            token_optimized_prompts: false,
            agent_retry_count: 1,
            agent_timeout_ms: 0,
        }
    }
}
