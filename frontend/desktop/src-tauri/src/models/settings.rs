use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::agents::{
    default_kimi_model, default_kimi_path, default_opencode_model, default_opencode_path,
    AgentBackend,
};

/// A single agent slot within a pipeline stage.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineAgent {
    pub provider: String,
    pub model: String,
}

/// Orchestrator agent that enhances prompts and routes to the right pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorSettings {
    pub agent: PipelineAgent,
    pub max_iterations: u32,
}

/// Configuration for the multi-stage code pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodePipelineSettings {
    pub planners: Vec<PipelineAgent>,
    pub coder: PipelineAgent,
    pub reviewers: Vec<PipelineAgent>,
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
    /// Per-provider enabled models (e.g. {"copilot": "claude-sonnet-4.6,gpt-5.4"}).
    #[serde(default)]
    pub provider_models: HashMap<String, String>,
    /// Port for the Symphony sidecar (0 = use default 8719).
    #[serde(default)]
    pub symphony_port: u16,
    /// Python interpreter path override (empty = auto-detect).
    #[serde(default)]
    pub python_path: String,
    /// Orchestrator configuration (None = not configured).
    #[serde(default)]
    pub orchestrator: Option<OrchestratorSettings>,
    /// Code pipeline configuration (None = not configured).
    #[serde(default)]
    pub code_pipeline: Option<CodePipelineSettings>,
    /// User interface language (reserved for future i18n).
    #[serde(default = "default_language")]
    pub language: String,
    /// Whether to prevent the system from sleeping whilst the app is open.
    #[serde(default)]
    pub keep_awake: bool,
    /// When to show OS completion notifications: "always", "never", "when_in_background".
    #[serde(default = "default_completion_notifications")]
    pub completion_notifications: String,
    /// Whether permission-request notifications are enabled.
    #[serde(default)]
    pub permission_notifications: bool,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_completion_notifications() -> String {
    "never".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            default_agent: None,
            claude_path: AgentBackend::Claude.default_path().to_string(),
            codex_path: AgentBackend::Codex.default_path().to_string(),
            gemini_path: AgentBackend::Gemini.default_path().to_string(),
            kimi_path: AgentBackend::Kimi.default_path().to_string(),
            opencode_path: AgentBackend::OpenCode.default_path().to_string(),
            claude_model: AgentBackend::Claude.default_model().to_string(),
            codex_model: AgentBackend::Codex.default_model().to_string(),
            gemini_model: AgentBackend::Gemini.default_model().to_string(),
            kimi_model: AgentBackend::Kimi.default_model().to_string(),
            opencode_model: AgentBackend::OpenCode.default_model().to_string(),
            provider_models: HashMap::new(),
            symphony_port: 0,
            python_path: String::new(),
            orchestrator: None,
            code_pipeline: None,
            language: default_language(),
            keep_awake: false,
            completion_notifications: default_completion_notifications(),
            permission_notifications: false,
        }
    }
}

impl AppSettings {
    pub fn is_supported_cli(cli_name: &str) -> bool {
        AgentBackend::from_cli_name(cli_name).is_some()
    }

    pub fn path_for_cli(&self, cli_name: &str) -> Option<&str> {
        Some(self.path_for_backend(AgentBackend::from_cli_name(cli_name)?))
    }

    pub fn model_csv_for_cli(&self, cli_name: &str) -> Option<&str> {
        self.model_csv_for_backend(AgentBackend::from_cli_name(cli_name)?)
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
        AgentBackend::from_cli_name(cli_name).map(AgentBackend::default_model)
    }

    pub(crate) fn path_for_backend(&self, backend: AgentBackend) -> &str {
        match backend {
            AgentBackend::Claude => self.claude_path.as_str(),
            AgentBackend::Codex => self.codex_path.as_str(),
            AgentBackend::Gemini => self.gemini_path.as_str(),
            AgentBackend::Kimi => self.kimi_path.as_str(),
            AgentBackend::OpenCode => self.opencode_path.as_str(),
            AgentBackend::Copilot => AgentBackend::Copilot.default_path(),
        }
    }

    pub(crate) fn model_csv_for_backend(&self, backend: AgentBackend) -> Option<&str> {
        match backend {
            AgentBackend::Claude => Some(self.claude_model.as_str()),
            AgentBackend::Codex => Some(self.codex_model.as_str()),
            AgentBackend::Gemini => Some(self.gemini_model.as_str()),
            AgentBackend::Kimi => Some(self.kimi_model.as_str()),
            AgentBackend::OpenCode => Some(self.opencode_model.as_str()),
            AgentBackend::Copilot => self
                .provider_models
                .get(backend.as_str())
                .map(String::as_str),
        }
    }
}
