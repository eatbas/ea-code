use serde::Serialize;
use serde_json::{Map, Value};

const CLAUDE_DEFAULT_MAX_TURNS: i64 = 200;

pub type SymphonyProviderOptions = Map<String, Value>;

/// Optional Kimi swarm configuration forwarded into `provider_options`.
pub struct KimiSwarmOptions {
    pub agent_file: String,
    /// Per-conversation swarm directory for generated files.
    pub swarm_dir: String,
    pub max_ralph_iterations: i32,
}

/// Shared request body for Symphony `/v1/chat` calls.
/// Used by both the single-turn chat path and the pipeline stage runner.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SymphonyChatRequest<'a> {
    pub provider: &'a str,
    pub model: &'a str,
    pub workspace_path: &'a str,
    pub mode: &'a str,
    pub prompt: &'a str,
    pub provider_session_ref: Option<&'a str>,
    pub provider_options: SymphonyProviderOptions,
}

/// Apply provider-specific defaults for Maestro-managed runs.
///
/// When `thinking_level` is `Some`, the value is forwarded to the
/// Symphony provider adapter as `"thinking_level"` in the options map.
pub fn default_provider_options(
    provider: &str,
    thinking_level: Option<&str>,
    kimi_swarm: Option<KimiSwarmOptions>,
) -> SymphonyProviderOptions {
    let mut options = SymphonyProviderOptions::new();
    if provider.eq_ignore_ascii_case("claude") {
        options.insert(
            "max_turns".to_string(),
            Value::Number(CLAUDE_DEFAULT_MAX_TURNS.into()),
        );
    }
    if let Some(level) = thinking_level {
        options.insert(
            "thinking_level".to_string(),
            Value::String(level.to_string()),
        );
    }
    if let Some(swarm) = kimi_swarm {
        options.insert("agent_file".to_string(), Value::String(swarm.agent_file));
        options.insert("swarm_dir".to_string(), Value::String(swarm.swarm_dir));
        options.insert(
            "max_ralph_iterations".to_string(),
            Value::Number(swarm.max_ralph_iterations.into()),
        );
    }
    options
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::default_provider_options;

    #[test]
    fn claude_defaults_raise_auto_mode_turn_limit() {
        let options = default_provider_options("claude", None, None);
        assert_eq!(options.get("max_turns"), Some(&Value::from(200)));
    }

    #[test]
    fn non_claude_defaults_are_empty() {
        assert!(default_provider_options("codex", None, None).is_empty());
    }

    #[test]
    fn thinking_level_is_forwarded() {
        let options = default_provider_options("claude", Some("medium"), None);
        assert_eq!(
            options.get("thinking_level"),
            Some(&Value::String("medium".to_string())),
        );
    }

    #[test]
    fn kimi_swarm_options_are_forwarded() {
        let swarm = super::KimiSwarmOptions {
            agent_file: "/tmp/swarm.yaml".to_string(),
            swarm_dir: "/tmp".to_string(),
            max_ralph_iterations: -1,
        };
        let options = default_provider_options("kimi", None, Some(swarm));
        assert_eq!(
            options.get("agent_file"),
            Some(&Value::String("/tmp/swarm.yaml".to_string())),
        );
        assert_eq!(options.get("max_ralph_iterations"), Some(&Value::from(-1)),);
    }
}
