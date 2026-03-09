/// MCP tool definitions exposed to connected agents.

use serde_json::{json, Value};

/// Returns the JSON array of tool definitions for the `tools/list` response.
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "get_session_history",
            "description": "Returns previous runs in the current session, including prompts, verdicts, and errors. Use this to understand what has already been attempted in this conversation thread.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to query. Defaults to the current session if omitted."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of runs to return (default: 10)."
                    }
                }
            }
        },
        {
            "name": "search_runs",
            "description": "Search past pipeline runs by prompt text or workspace path. Useful for finding previous attempts at similar tasks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for in run prompts."
                    },
                    "workspace_path": {
                        "type": "string",
                        "description": "Filter by workspace path."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 10)."
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "get_run_output",
            "description": "Returns full output and artefacts (diffs, reviews, judge verdicts) for a specific run. Use a run ID obtained from get_session_history or search_runs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "run_id": {
                        "type": "string",
                        "description": "The run ID to retrieve."
                    }
                },
                "required": ["run_id"]
            }
        },
        {
            "name": "get_project_summary",
            "description": "Returns project info and recent run statistics for a workspace path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workspace_path": {
                        "type": "string",
                        "description": "Filesystem path of the project workspace."
                    }
                },
                "required": ["workspace_path"]
            }
        }
    ])
}
