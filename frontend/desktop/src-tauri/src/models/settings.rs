use serde::{Deserialize, Serialize};

use super::agents::{
    default_executive_summary_model, default_kimi_model, default_kimi_path, default_opencode_model,
    default_opencode_path, AgentBackend,
};

pub const AI_CLI_NAMES: [&str; 5] = ["claude", "codex", "gemini", "kimi", "opencode"];

/// Configuration for an extra parallel planner or reviewer slot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraSlotConfig {
    pub agent: Option<AgentBackend>,
    pub model: Option<String>,
}

/// Application settings persisted locally.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub default_agent: Option<String>,
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
    /// Review Merger agent backend (activates when 2+ reviewers configured).
    #[serde(default)]
    pub review_merger_agent: Option<AgentBackend>,
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
    /// Model for review merger stage.
    #[serde(default)]
    pub review_merger_model: Option<String>,
    pub code_fixer_model: String,
    pub final_judge_model: String,
    #[serde(default)]
    pub executive_summary_agent: Option<AgentBackend>,
    #[serde(default = "default_executive_summary_model")]
    pub executive_summary_model: String,
    /// Budget mode: skip all planning stages, send prompt directly to coder.
    #[serde(default)]
    pub budget_mode: bool,
    /// Minimum weighted review score to pass (default 7.0).
    #[serde(default = "default_review_pass_score")]
    pub review_pass_score: f64,
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
    pub token_optimised_prompts: bool,
    /// Number of retries per agent call on failure (0 = no retry).
    #[serde(default = "default_agent_retry_count")]
    pub agent_retry_count: u32,
    /// Per-agent timeout in milliseconds (0 = no timeout).
    #[serde(default = "default_agent_timeout_ms")]
    pub agent_timeout_ms: u64,
    /// Maximum agentic turns per invocation for CLIs that support it.
    #[serde(default = "default_agent_max_turns")]
    pub agent_max_turns: u32,
    /// Completed runs older than this many days are deleted on startup (0 = disabled).
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    // --- Parametric parallel slots ---
    /// Extra planner slot configurations (planner 2, 3, 4, ...).
    #[serde(default)]
    pub extra_planners: Vec<ExtraSlotConfig>,
    /// Extra reviewer slot configurations (reviewer 2, 3, 4, ...).
    #[serde(default)]
    pub extra_reviewers: Vec<ExtraSlotConfig>,
    /// Maximum total planner slots (1 = primary only, 2+ = primary + extras). Default 4.
    #[serde(default = "default_max_planners")]
    pub max_planners: u32,
    /// Maximum total reviewer slots (1 = primary only, 2+ = primary + extras). Default 4.
    #[serde(default = "default_max_reviewers")]
    pub max_reviewers: u32,

    // --- Legacy fields for backward-compatible migration (read-only) ---
    /// Deprecated: use extra_planners[0] instead.
    #[serde(default, skip_serializing)]
    pub planner_2_agent: Option<AgentBackend>,
    /// Deprecated: use extra_planners[1] instead.
    #[serde(default, skip_serializing)]
    pub planner_3_agent: Option<AgentBackend>,
    /// Deprecated: use extra_planners[0].model instead.
    #[serde(default, skip_serializing)]
    pub planner_2_model: Option<String>,
    /// Deprecated: use extra_planners[1].model instead.
    #[serde(default, skip_serializing)]
    pub planner_3_model: Option<String>,
    /// Deprecated: use extra_reviewers[0] instead.
    #[serde(default, skip_serializing)]
    pub code_reviewer_2_agent: Option<AgentBackend>,
    /// Deprecated: use extra_reviewers[1] instead.
    #[serde(default, skip_serializing)]
    pub code_reviewer_3_agent: Option<AgentBackend>,
    /// Deprecated: use extra_reviewers[0].model instead.
    #[serde(default, skip_serializing)]
    pub code_reviewer_2_model: Option<String>,
    /// Deprecated: use extra_reviewers[1].model instead.
    #[serde(default, skip_serializing)]
    pub code_reviewer_3_model: Option<String>,
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

fn default_agent_timeout_ms() -> u64 {
    600_000 // 10 minutes
}

fn default_agent_max_turns() -> u32 {
    25
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_retention_days() -> u32 {
    90
}

fn default_review_pass_score() -> f64 {
    7.0
}

fn default_max_planners() -> u32 {
    4
}

fn default_max_reviewers() -> u32 {
    4
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            default_agent: None,
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
            review_merger_agent: None,
            code_fixer_agent: None,
            final_judge_agent: None,
            max_iterations: 3,
            require_git: true,
            claude_model: "sonnet".to_string(),
            codex_model: "gpt-5.3-codex".to_string(),
            gemini_model: "gemini-3-flash-preview".to_string(),
            kimi_model: "kimi-code/kimi-for-coding".to_string(),
            opencode_model: "opencode/glm-5".to_string(),
            prompt_enhancer_model: "sonnet".to_string(),
            skill_selector_model: None,
            planner_model: None,
            plan_auditor_model: None,
            coder_model: "sonnet".to_string(),
            code_reviewer_model: "gpt-5.3-codex".to_string(),
            review_merger_model: None,
            code_fixer_model: "sonnet".to_string(),
            final_judge_model: "gpt-5.3-codex".to_string(),
            executive_summary_agent: None,
            executive_summary_model: "gpt-5.3-codex".to_string(),
            budget_mode: false,
            review_pass_score: 7.0,
            require_plan_approval: false,
            plan_auto_approve_timeout_sec: 45,
            max_plan_revisions: 3,
            token_optimised_prompts: false,
            agent_retry_count: 1,
            agent_timeout_ms: 600_000,
            agent_max_turns: 25,
            retention_days: 90,
            skill_selector_agent: None,
            extra_planners: Vec::new(),
            extra_reviewers: Vec::new(),
            max_planners: 4,
            max_reviewers: 4,
            // Legacy fields
            planner_2_agent: None,
            planner_3_agent: None,
            planner_2_model: None,
            planner_3_model: None,
            code_reviewer_2_agent: None,
            code_reviewer_3_agent: None,
            code_reviewer_2_model: None,
            code_reviewer_3_model: None,
        }
    }
}

impl AppSettings {
    /// Migrates legacy individual slot fields into the new array-based format
    /// and truncates arrays to the configured max. Call after deserialization.
    pub fn normalize(&mut self) {
        // Migrate legacy planner fields into extra_planners if arrays are empty.
        if self.extra_planners.is_empty() {
            let legacy = [
                (&self.planner_2_agent, &self.planner_2_model),
                (&self.planner_3_agent, &self.planner_3_model),
            ];
            for (agent, model) in &legacy {
                if agent.is_some() || model.is_some() {
                    self.extra_planners.push(ExtraSlotConfig {
                        agent: (*agent).clone(),
                        model: (*model).clone(),
                    });
                }
            }
        }

        // Migrate legacy reviewer fields into extra_reviewers if arrays are empty.
        if self.extra_reviewers.is_empty() {
            let legacy = [
                (&self.code_reviewer_2_agent, &self.code_reviewer_2_model),
                (&self.code_reviewer_3_agent, &self.code_reviewer_3_model),
            ];
            for (agent, model) in &legacy {
                if agent.is_some() || model.is_some() {
                    self.extra_reviewers.push(ExtraSlotConfig {
                        agent: (*agent).clone(),
                        model: (*model).clone(),
                    });
                }
            }
        }

        // Clear legacy fields.
        self.planner_2_agent = None;
        self.planner_3_agent = None;
        self.planner_2_model = None;
        self.planner_3_model = None;
        self.code_reviewer_2_agent = None;
        self.code_reviewer_3_agent = None;
        self.code_reviewer_2_model = None;
        self.code_reviewer_3_model = None;

        // Truncate arrays to the configured max (max - 1 extra slots).
        let max_extra_planners = self.max_planners.saturating_sub(1) as usize;
        if self.extra_planners.len() > max_extra_planners {
            self.extra_planners.truncate(max_extra_planners);
        }
        let max_extra_reviewers = self.max_reviewers.saturating_sub(1) as usize;
        if self.extra_reviewers.len() > max_extra_reviewers {
            self.extra_reviewers.truncate(max_extra_reviewers);
        }
    }

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
            "gemini" => Some("gemini-3-flash-preview"),
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

    /// Returns the number of active planner slots (0 = none, 1 = primary only, etc.).
    pub fn active_planner_count(&self) -> usize {
        let primary = if self.planner_agent.is_some() { 1 } else { 0 };
        let extras = self
            .extra_planners
            .iter()
            .filter(|s| s.agent.is_some())
            .count();
        primary + extras
    }

    /// Returns the number of active reviewer slots (1 = primary only, etc.).
    pub fn active_reviewer_count(&self) -> usize {
        let primary = if self.code_reviewer_agent.is_some() {
            1
        } else {
            0
        };
        let extras = self
            .extra_reviewers
            .iter()
            .filter(|s| s.agent.is_some())
            .count();
        primary + extras
    }
}
