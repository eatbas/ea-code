use std::collections::HashMap;

use crate::models::McpRuntimeStatus;

use super::BUILTIN_SERVER_IDS;

/// Parses CLI output (`mcp list`) into a map of server_id → status.
///
/// Returns `Some(map)` when the output is recognisable — even if the map is empty
/// (meaning the CLI has no MCP servers configured, i.e. all are Disabled).
/// Returns `None` only when the output is completely unparseable.
pub(super) fn parse_runtime_map(raw: &str) -> Option<HashMap<String, McpRuntimeStatus>> {
    let mut map = HashMap::<String, McpRuntimeStatus>::new();

    // Attempt 1: JSON output (e.g. codex --json, some future CLIs).
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) {
        for server_id in BUILTIN_SERVER_IDS {
            if let Some(status) = infer_status_from_json(&json, server_id) {
                map.insert(server_id.to_string(), status);
            }
        }
    }

    // Attempt 2: plaintext line-by-line.
    if map.is_empty() {
        for server_id in BUILTIN_SERVER_IDS {
            if let Some(status) = infer_status_from_text(raw, server_id) {
                map.insert(server_id.to_string(), status);
            }
        }
    }

    // If no built-in servers were found, check whether the output is a recognisable
    // "no servers" message rather than garbage. If it is, return an empty map
    // (all servers Disabled) instead of None (which signals a parse error).
    if map.is_empty() {
        let stripped = strip_ansi(raw);
        let lower = stripped.to_lowercase();
        if lower.contains("no mcp") || lower.contains("no servers") || lower.contains("0 server") {
            return Some(map);
        }
        // If the CLI produced recognisable structured output (e.g. table headers, "mcp"
        // keyword) but none of our server IDs matched, that's a valid empty list.
        if lower.contains("mcp") && lower.contains("server") {
            return Some(map);
        }
        return None;
    }

    Some(map)
}

fn infer_status_from_json(value: &serde_json::Value, server_id: &str) -> Option<McpRuntimeStatus> {
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

/// Searches line-by-line for a server_id mention + status keywords.
///
/// Recognised positive keywords: enabled, active, running, connected.
/// Recognised negative keywords: disabled, inactive, off.
/// If the server_id is found on a line but no keyword matches, we assume Enabled
/// (the CLI listed it, so it must be configured).
fn infer_status_from_text(raw: &str, server_id: &str) -> Option<McpRuntimeStatus> {
    let stripped = strip_ansi(raw);
    let needle = server_id.trim().to_lowercase();
    for line in stripped.lines() {
        let lower = line.to_lowercase();
        if !needle.is_empty() && !lower.contains(&needle) {
            continue;
        }
        if lower.contains("disabled") || lower.contains("inactive") || lower.contains("off") {
            return Some(McpRuntimeStatus::Disabled);
        }
        if lower.contains("enabled")
            || lower.contains("active")
            || lower.contains("running")
            || lower.contains("connected")
        {
            return Some(McpRuntimeStatus::Enabled);
        }
        // Line mentions the server but no explicit keyword → treat as Enabled.
        if !needle.is_empty() {
            return Some(McpRuntimeStatus::Enabled);
        }
    }
    None
}

/// Strips ANSI escape codes (e.g. `\x1b[90m`) from CLI output.
/// Many CLIs (opencode, kimi) emit coloured output even when piped.
fn strip_ansi(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip ESC + '[' + parameters + final byte.
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}
