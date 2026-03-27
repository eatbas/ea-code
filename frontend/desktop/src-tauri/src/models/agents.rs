use serde::{Deserialize, Serialize};

/// Supported CLI agent backends.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AgentBackend {
    Claude,
    Codex,
    Gemini,
    Kimi,
    OpenCode,
    Copilot,
}

impl AgentBackend {
    pub(crate) const MANAGED: [Self; 5] = [
        Self::Claude,
        Self::Codex,
        Self::Gemini,
        Self::Kimi,
        Self::OpenCode,
    ];

    pub(crate) fn from_cli_name(cli_name: &str) -> Option<Self> {
        match cli_name {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "gemini" => Some(Self::Gemini),
            "kimi" => Some(Self::Kimi),
            "opencode" => Some(Self::OpenCode),
            "copilot" => Some(Self::Copilot),
            _ => None,
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Kimi => "kimi",
            Self::OpenCode => "opencode",
            Self::Copilot => "copilot",
        }
    }

    pub(crate) const fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude CLI",
            Self::Codex => "Codex CLI",
            Self::Gemini => "Gemini CLI",
            Self::Kimi => "Kimi CLI",
            Self::OpenCode => "OpenCode CLI",
            Self::Copilot => "Copilot CLI",
        }
    }

    pub(crate) const fn default_path(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Kimi => "kimi",
            Self::OpenCode => "opencode",
            Self::Copilot => "copilot",
        }
    }

    pub(crate) const fn default_model(self) -> &'static str {
        match self {
            Self::Claude => "sonnet",
            Self::Codex => "gpt-5.3-codex",
            Self::Gemini => "gemini-3-flash-preview",
            Self::Kimi => "kimi-code/kimi-for-coding",
            Self::OpenCode => "opencode/glm-5",
            Self::Copilot => "claude-sonnet-4.6",
        }
    }

    pub(crate) const fn package_name(self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("@anthropic-ai/claude-code"),
            Self::Codex => Some("@openai/codex"),
            Self::Gemini => Some("@google/gemini-cli"),
            Self::Kimi => Some("kimi-cli"),
            Self::OpenCode => Some("opencode-ai"),
            Self::Copilot => None,
        }
    }
}

pub(crate) fn default_kimi_path() -> String {
    AgentBackend::Kimi.default_path().to_string()
}

pub(crate) fn default_opencode_path() -> String {
    AgentBackend::OpenCode.default_path().to_string()
}

pub(crate) fn default_kimi_model() -> String {
    AgentBackend::Kimi.default_model().to_string()
}

pub(crate) fn default_opencode_model() -> String {
    AgentBackend::OpenCode.default_model().to_string()
}

#[cfg(test)]
mod tests {
    use super::AgentBackend;

    #[test]
    fn managed_backends_all_have_package_metadata() {
        assert!(AgentBackend::MANAGED
            .into_iter()
            .all(|backend| backend.package_name().is_some()));
    }

    #[test]
    fn cli_name_mapping_round_trips() {
        for backend in [
            AgentBackend::Claude,
            AgentBackend::Codex,
            AgentBackend::Gemini,
            AgentBackend::Kimi,
            AgentBackend::OpenCode,
            AgentBackend::Copilot,
        ] {
            assert_eq!(AgentBackend::from_cli_name(backend.as_str()), Some(backend));
        }
    }
}
