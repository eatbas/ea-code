use std::fs::OpenOptions;
use std::io::Write;

use crate::storage::with_conversations_lock;

use super::paths::pipeline_debug_file_path;

pub fn read_pipeline_debug_log(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<String, String> {
    with_conversations_lock(|| {
        let path = pipeline_debug_file_path(workspace_path, conversation_id);
        if !path.exists() {
            return Ok(String::new());
        }

        std::fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read pipeline debug log {}: {error}", path.display()))
    })
}

pub fn append_pipeline_debug_log(
    workspace_path: &str,
    conversation_id: &str,
    line: &str,
) -> Result<(), String> {
    with_conversations_lock(|| {
        let path = pipeline_debug_file_path(workspace_path, conversation_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create debug log directory {}: {error}", parent.display()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("Failed to open pipeline debug log {}: {error}", path.display()))?;

        writeln!(file, "{line}")
            .map_err(|error| format!("Failed to write pipeline debug log {}: {error}", path.display()))
    })
}
