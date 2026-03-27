use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::agents::{default_kimi_model, default_kimi_path, default_opencode_model, default_opencode_path};

pub const AI_CLI_NAMES: [&str; 6] = ["claude", "codex", "gemini", "kimi", "opencode", "copilot"];

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
    /// Port for the hive-api sidecar (0 = use default 8719).
    #[serde(default)]
    pub hive_api_port: u16,
    /// Python interpreter path override (empty = auto-detect).
    #[serde(default)]
    pub python_path: String,
}

fn default_theme() -> String {
    "system".to_string()
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
            claude_model: "sonnet".to_string(),
            codex_model: "gpt-5.3-codex".to_string(),
            gemini_model: "gemini-3-flash-preview".to_string(),
            kimi_model: "kimi-code/kimi-for-coding".to_string(),
            opencode_model: "opencode/glm-5".to_string(),
            provider_models: HashMap::new(),
            hive_api_port: 0,
            python_path: String::new(),
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
            "copilot" => Some("copilot"),
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
            other => self.provider_models.get(other).map(|s| s.as_str()),
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
            "copilot" => Some("claude-sonnet-4.6"),
            _ => None,
        }
    }
}
