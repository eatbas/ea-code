use serde::Serialize;
use serde_json::{Map, Value};

const CLAUDE_DEFAULT_MAX_TURNS: i64 = 200;

pub type SymphonyProviderOptions = Map<String, Value>;

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
) -> SymphonyProviderOptions {
    let mut options = SymphonyProviderOptions::new();
    if provider.eq_ignore_ascii_case("claude") {
        options.insert(
            "max_turns".to_string(),
            Value::Number(CLAUDE_DEFAULT_MAX_TURNS.into()),
        );
    }
    if let Some(level) = thinking_level {
        options.insert("thinking_level".to_string(), Value::String(level.to_string()));
    }
    options
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::default_provider_options;

    #[test]
    fn claude_defaults_raise_auto_mode_turn_limit() {
        let options = default_provider_options("claude", None);
        assert_eq!(options.get("max_turns"), Some(&Value::from(200)));
    }

    #[test]
    fn non_claude_defaults_are_empty() {
        assert!(default_provider_options("codex", None).is_empty());
    }

    #[test]
    fn thinking_level_is_forwarded() {
        let options = default_provider_options("claude", Some("medium"));
        assert_eq!(
            options.get("thinking_level"),
            Some(&Value::String("medium".to_string())),
        );
    }
}
