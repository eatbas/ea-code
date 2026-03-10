use std::collections::HashMap;

use crate::models::McpRuntimeStatus;

use super::BUILTIN_SERVER_IDS;

pub(super) fn parse_runtime_map(raw: &str) -> Option<HashMap<String, McpRuntimeStatus>> {
    let mut map = HashMap::<String, McpRuntimeStatus>::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) {
        for server_id in BUILTIN_SERVER_IDS {
            if let Some(status) = infer_status_from_json(&json, server_id) {
                map.insert(server_id.to_string(), status);
            }
        }
    }

    if map.is_empty() {
        for server_id in BUILTIN_SERVER_IDS {
            if let Some(status) = infer_status_from_text(raw, server_id) {
                map.insert(server_id.to_string(), status);
            }
        }
    }

    if map.is_empty() { None } else { Some(map) }
}

fn infer_status_from_json(
    value: &serde_json::Value,
    server_id: &str,
) -> Option<McpRuntimeStatus> {
    match value {
        serde_json::Value::Object(obj) => {
            if let Some(direct) = obj.get(server_id) {
                return infer_explicit_status(direct).or(Some(McpRuntimeStatus::Enabled));
            }
            for child in obj.values() {
                if let Some(status) = infer_status_from_json(child, server_id) {
                    return Some(status);
                }
            }
            None
        }
        serde_json::Value::Array(items) => items
            .iter()
            .find_map(|item| infer_status_from_json(item, server_id)),
        serde_json::Value::String(text) => infer_status_from_text(text, server_id),
        _ => None,
    }
}

fn infer_explicit_status(value: &serde_json::Value) -> Option<McpRuntimeStatus> {
    match value {
        serde_json::Value::Bool(flag) => Some(if *flag {
            McpRuntimeStatus::Enabled
        } else {
            McpRuntimeStatus::Disabled
        }),
        serde_json::Value::Object(obj) => {
            if let Some(serde_json::Value::Bool(flag)) =
                obj.get("enabled").or_else(|| obj.get("isEnabled"))
            {
                return Some(if *flag {
                    McpRuntimeStatus::Enabled
                } else {
                    McpRuntimeStatus::Disabled
                });
            }
            if let Some(serde_json::Value::String(state)) =
                obj.get("status").or_else(|| obj.get("state"))
            {
                return infer_status_from_text(state, "");
            }
            None
        }
        serde_json::Value::String(text) => infer_status_from_text(text, ""),
        _ => None,
    }
}

fn infer_status_from_text(raw: &str, server_id: &str) -> Option<McpRuntimeStatus> {
    let needle = server_id.trim().to_lowercase();
    for line in raw.lines() {
        let lower = line.to_lowercase();
        if !needle.is_empty() && !lower.contains(&needle) {
            continue;
        }
        if lower.contains("disabled") || lower.contains("inactive") || lower.contains("off") {
            return Some(McpRuntimeStatus::Disabled);
        }
        if lower.contains("enabled") || lower.contains("active") || lower.contains("running") {
            return Some(McpRuntimeStatus::Enabled);
        }
        if !needle.is_empty() {
            return Some(McpRuntimeStatus::Enabled);
        }
    }
    None
}
