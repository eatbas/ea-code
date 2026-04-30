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

fn thinking_key_for_provider(provider: &str) -> &'static str {
    if provider.eq_ignore_ascii_case("kimi") || provider.eq_ignore_ascii_case("opencode") {
        "thinking_mode"
    } else {
        "thinking_level"
    }
}

fn normalise_thinking_value(provider: &str, value: &str) -> String {
    if provider.eq_ignore_ascii_case("kimi") || provider.eq_ignore_ascii_case("opencode") {
        match value {
            "on" => "enabled".to_string(),
            "off" => "disabled".to_string(),
            _ => value.to_string(),
        }
    } else {
        value.to_string()
    }
}

/// Returns whether the (provider, model) pair accepts a thinking level
/// or thinking mode option. Mirrors the per-adapter logic in Symphony
/// (`ClaudeAdapter._effort_levels_for_model`, `CodexAdapter`, etc.) so
/// we never forward an option the model would silently ignore -- and,
/// critically, never forward one that older Symphony versions would
/// reject with `ValueError`, killing the worker.
///
/// Keep this list in sync with the adapter schemas in
/// `symphony-api/src/symphony/providers/`. The integration test in
/// `tests/test_provider_options.rs` exercises the round trip.
pub fn model_supports_thinking(provider: &str, model: &str) -> bool {
    let model_lc = model.to_lowercase();
    match provider.to_lowercase().as_str() {
        "claude" => {
            // Haiku has no thinking-level controls; only opus / sonnet
            // (and unknown future names defaulting to true) do.
            !model_lc.contains("haiku")
        }
        // Codex, kimi, and opencode adapters expose thinking options
        // for every configured model today.
        "codex" | "kimi" | "opencode" => true,
        // Gemini and Copilot adapters do not expose any thinking option
        // schema -- forwarding one would either be ignored (latest
        // Symphony) or rejected (older Symphony).
        "gemini" | "copilot" => false,
        // Unknown provider -- be permissive; Symphony will validate.
        _ => true,
    }
}

/// Apply provider-specific defaults for Maestro-managed runs.
///
/// When `thinking_level` is `Some` and the (provider, model) pair
/// actually supports a thinking option, the value is forwarded under
/// the correct key (`thinking_level` or `thinking_mode`). For models
/// that do not support thinking (e.g. Claude Haiku, Gemini, Copilot)
/// the option is dropped entirely so Symphony never sees an option it
/// would have to ignore -- or, on older Symphony builds, reject and
/// crash the worker over.
pub fn default_provider_options(
    provider: &str,
    model: &str,
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
        if model_supports_thinking(provider, model) {
            let key = thinking_key_for_provider(provider);
            let value = normalise_thinking_value(provider, level);
            options.insert(key.to_string(), Value::String(value));
        }
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

    use super::{default_provider_options, model_supports_thinking};

    #[test]
    fn claude_defaults_raise_auto_mode_turn_limit() {
        let options = default_provider_options("claude", "opus", None, None);
        assert_eq!(options.get("max_turns"), Some(&Value::from(200)));
    }

    #[test]
    fn non_claude_defaults_are_empty() {
        assert!(default_provider_options("codex", "gpt-5.4", None, None).is_empty());
    }

    #[test]
    fn thinking_level_is_forwarded_for_supported_claude_models() {
        let options = default_provider_options("claude", "opus", Some("medium"), None);
        assert_eq!(
            options.get("thinking_level"),
            Some(&Value::String("medium".to_string())),
        );
    }

    /// Regression: previously the desktop app forwarded
    /// ``thinking_level`` for haiku because the user's settings still
    /// had the stale ``"claude:haiku": "medium"`` entry. On older
    /// Symphony builds that would raise ``ValueError`` inside the
    /// musician's worker task, kill the loop, and strand every future
    /// haiku score in the queue with no consumer -- which manifested
    /// in the UI as "Prompt Enhancer: Waiting for output..." forever.
    #[test]
    fn thinking_level_is_dropped_for_claude_haiku() {
        let options = default_provider_options("claude", "haiku", Some("medium"), None);
        assert!(options.get("thinking_level").is_none());
        assert!(options.get("thinking_mode").is_none());
        // max_turns is independent of thinking and must still be set.
        assert_eq!(options.get("max_turns"), Some(&Value::from(200)));
    }

    #[test]
    fn thinking_level_is_dropped_for_gemini_and_copilot() {
        for provider in ["gemini", "copilot"] {
            let options = default_provider_options(provider, "any-model", Some("high"), None);
            assert!(
                options.get("thinking_level").is_none(),
                "{provider} should not receive thinking_level",
            );
            assert!(
                options.get("thinking_mode").is_none(),
                "{provider} should not receive thinking_mode",
            );
        }
    }

    #[test]
    fn kimi_thinking_uses_mode_key() {
        let options = default_provider_options("kimi", "kimi-for-coding", Some("enabled"), None);
        assert_eq!(
            options.get("thinking_mode"),
            Some(&Value::String("enabled".to_string())),
        );
        assert!(options.get("thinking_level").is_none());
    }

    #[test]
    fn kimi_thinking_normalises_legacy_values() {
        let on_opts = default_provider_options("kimi", "kimi-for-coding", Some("on"), None);
        assert_eq!(
            on_opts.get("thinking_mode"),
            Some(&Value::String("enabled".to_string())),
        );

        let off_opts = default_provider_options("kimi", "kimi-for-coding", Some("off"), None);
        assert_eq!(
            off_opts.get("thinking_mode"),
            Some(&Value::String("disabled".to_string())),
        );
    }

    #[test]
    fn opencode_thinking_uses_mode_key() {
        let options = default_provider_options("opencode", "glm-5", Some("disabled"), None);
        assert_eq!(
            options.get("thinking_mode"),
            Some(&Value::String("disabled".to_string())),
        );
    }

    #[test]
    fn kimi_swarm_options_are_forwarded() {
        let swarm = super::KimiSwarmOptions {
            agent_file: "/tmp/swarm.yaml".to_string(),
            swarm_dir: "/tmp".to_string(),
            max_ralph_iterations: -1,
        };
        let options = default_provider_options("kimi", "kimi-for-coding", None, Some(swarm));
        assert_eq!(
            options.get("agent_file"),
            Some(&Value::String("/tmp/swarm.yaml".to_string())),
        );
        assert_eq!(options.get("max_ralph_iterations"), Some(&Value::from(-1)),);
    }

    #[test]
    fn model_supports_thinking_matches_symphony_adapter_logic() {
        // Claude: haiku is the only currently-known unsupported model.
        assert!(!model_supports_thinking("claude", "haiku"));
        assert!(!model_supports_thinking("Claude", "Claude-Haiku-4.5"));
        assert!(model_supports_thinking("claude", "opus"));
        assert!(model_supports_thinking("claude", "opus[1m]"));
        assert!(model_supports_thinking("claude", "sonnet"));
        // Other providers -- whole-provider on/off rules.
        assert!(model_supports_thinking("codex", "gpt-5.5"));
        assert!(model_supports_thinking("kimi", "kimi-for-coding"));
        assert!(model_supports_thinking("opencode", "glm-5"));
        assert!(!model_supports_thinking("gemini", "gemini-3-pro-preview"));
        assert!(!model_supports_thinking("copilot", "claude-haiku-4.5"));
    }
}
