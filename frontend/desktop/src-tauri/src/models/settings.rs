use serde::{Deserialize, Serialize};

use super::agents::{
    default_executive_summary_model, default_kimi_model, default_kimi_path,
    default_opencode_model, default_opencode_path, AgentBackend,
};

pub const AI_CLI_NAMES: [&str; 5] = ["claude", "codex", "gemini", "kimi", "opencode"];

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
    #[serde(default)]
    pub prompt_enhancer_agent: Option<AgentBackend>,
    #[serde(default)]
    pub skill_selector_agent: Option<AgentBackend>,
    #[serde(default)]
    pub planner_agent: Option<AgentBackend>,
    #[serde(default)]
    pub plan_auditor_agent: Option<AgentBackend>,
    #[serde(default)]
    pub coder_agent: Option<AgentBackend>,
    #[serde(default)]
    pub code_reviewer_agent: Option<AgentBackend>,
    #[serde(default)]
    pub code_fixer_agent: Option<AgentBackend>,
    #[serde(default)]
    pub final_judge_agent: Option<AgentBackend>,
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
    pub skill_selector_model: Option<String>,
    #[serde(default)]
    pub planner_model: Option<String>,
    #[serde(default)]
    pub plan_auditor_model: Option<String>,
    pub coder_model: String,
    pub code_reviewer_model: String,
    pub code_fixer_model: String,
    pub final_judge_model: String,
    #[serde(default)]
    pub executive_summary_agent: Option<AgentBackend>,
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
    /// Maximum agentic turns per invocation for CLIs that support it.
    #[serde(default = "default_agent_max_turns")]
    pub agent_max_turns: u32,
    /// Completed runs older than this many days are deleted on startup (0 = disabled).
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
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

fn default_agent_max_turns() -> u32 {
    25
}

fn default_retention_days() -> u32 {
    90
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            codex_path: "codex".to_string(),
            gemini_path: "gemini".to_string(),
            kimi_path: "kimi".to_string(),
            opencode_path: "opencode".to_string(),
            prompt_enhancer_agent: None,
            planner_agent: None,
            plan_auditor_agent: None,
            coder_agent: None,
            code_reviewer_agent: None,
            code_fixer_agent: None,
            final_judge_agent: None,
            max_iterations: 3,
            require_git: true,
            claude_model: "sonnet".to_string(),
            codex_model: "gpt-5.3-codex".to_string(),
            gemini_model: "gemini-2.5-pro".to_string(),
            kimi_model: "kimi-code/kimi-for-coding".to_string(),
            opencode_model: "opencode/glm-5".to_string(),
            prompt_enhancer_model: "sonnet".to_string(),
            skill_selector_model: None,
            planner_model: None,
            plan_auditor_model: None,
            coder_model: "sonnet".to_string(),
            code_reviewer_model: "gpt-5.3-codex".to_string(),
            code_fixer_model: "sonnet".to_string(),
            final_judge_model: "gpt-5.3-codex".to_string(),
            executive_summary_agent: None,
            executive_summary_model: "gpt-5.3-codex".to_string(),
            require_plan_approval: false,
            plan_auto_approve_timeout_sec: 45,
            max_plan_revisions: 3,
            token_optimized_prompts: false,
            agent_retry_count: 1,
            agent_timeout_ms: 0,
            agent_max_turns: 25,
            retention_days: 90,
            skill_selector_agent: None,
        }
    }
}

impl AppSettings {
    pub fn is_supported_cli(cli_name: &str) -> bool {
        AI_CLI_NAMES.contains(&cli_name)
    }

    pub fn path_for_cli(&self, cli_name: &str) -> Option<&str> {
        match cli_name {
            "claude" => Some(self.claude_path.as_str()),
            "codex" => Some(self.codex_path.as_str()),
            "gemini" => Some(self.gemini_path.as_str()),
            "kimi" => Some(self.kimi_path.as_str()),
            "opencode" => Some(self.opencode_path.as_str()),
            _ => None,
        }
    }

    pub fn model_csv_for_cli(&self, cli_name: &str) -> Option<&str> {
        match cli_name {
            "claude" => Some(self.claude_model.as_str()),
            "codex" => Some(self.codex_model.as_str()),
            "gemini" => Some(self.gemini_model.as_str()),
            "kimi" => Some(self.kimi_model.as_str()),
            "opencode" => Some(self.opencode_model.as_str()),
            _ => None,
        }
    }

    pub fn primary_model_for_cli(&self, cli_name: &str) -> Option<String> {
        let csv = self.model_csv_for_cli(cli_name)?;
        let first = csv.split(',').next().unwrap_or("").trim();
        if first.is_empty() {
            None
        } else {
            Some(first.to_string())
        }
    }

    pub fn default_model_for_cli(cli_name: &str) -> Option<&'static str> {
        match cli_name {
            "claude" => Some("sonnet"),
            "codex" => Some("gpt-5.3-codex"),
            "gemini" => Some("gemini-2.5-pro"),
            "kimi" => Some("kimi-code"),
            "opencode" => Some("opencode/glm-5"),
            _ => None,
        }
    }

    pub fn missing_minimum_agents(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();

        if self.prompt_enhancer_agent.is_none() {
            missing.push("Prompt Enhancer");
        }
        if self.coder_agent.is_none() {
            missing.push("Coder");
        }
        if self.code_reviewer_agent.is_none() {
            missing.push("Code Reviewer");
        }
        if self.code_fixer_agent.is_none() {
            missing.push("Code Fixer");
        }
        if self.final_judge_agent.is_none() {
            missing.push("Judge");
        }
        if self.executive_summary_agent.is_none() {
            missing.push("Executive Summary");
        }

        missing
    }
}
