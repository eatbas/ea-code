/// JSON-RPC 2.0 response helpers and request dispatch logic.

use serde_json::{json, Value};

use ea_code_lib::db::DbPool;

use super::handlers;
use super::tools::tool_definitions;

/// Builds a successful JSON-RPC response.
pub fn make_response(id: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

/// Builds a JSON-RPC error response.
pub fn make_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

/// Wraps text into an MCP tool-result content block.
pub fn make_tool_result(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

/// Dispatches a JSON-RPC request to the appropriate handler.
pub fn handle_request(pool: &DbPool, request: &Value, default_session: &Option<String>) -> Value {
    let id = request.get("id").unwrap_or(&Value::Null);
    let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");

    match method {
        "initialize" => make_response(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ea-code-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        ),

        "notifications/initialized" => {
            // Notification — no response needed
            Value::Null
        }

        "tools/list" => make_response(id, json!({ "tools": tool_definitions() })),

        "tools/call" => {
            let params = request.get("params").unwrap_or(&Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let empty_args = json!({});
            let arguments = params.get("arguments").unwrap_or(&empty_args);

            let result = match tool_name {
                "get_session_history" => {
                    handlers::handle_get_session_history(pool, arguments, default_session)
                }
                "search_runs" => handlers::handle_search_runs(pool, arguments),
                "get_run_output" => handlers::handle_get_run_output(pool, arguments),
                "get_project_summary" => handlers::handle_get_project_summary(pool, arguments),
                _ => Err(format!("Unknown tool: {tool_name}")),
            };

            match result {
                Ok(data) => {
                    let text = serde_json::to_string_pretty(&data).unwrap_or_default();
                    make_response(id, make_tool_result(&text, false))
                }
                Err(e) => make_response(id, make_tool_result(&e, true)),
            }
        }

        // Ignore unknown notifications (no id = notification)
        _ if id.is_null() => Value::Null,

        _ => make_error(id, -32601, &format!("Method not found: {method}")),
    }
}
