use std::collections::HashMap;

use serde::Serialize;

/// Shared request body for hive-api `/v1/chat` calls.
/// Used by both the single-turn chat path and the pipeline stage runner.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct HiveChatRequest<'a> {
    pub provider: &'a str,
    pub model: &'a str,
    pub workspace_path: &'a str,
    pub mode: &'a str,
    pub prompt: &'a str,
    pub provider_session_ref: Option<&'a str>,
    pub stream: bool,
    pub provider_options: HashMap<String, String>,
}
