//! CLI session persistence for pipeline runs.
//!
//! Stores hive-api session references per run so stages within the same
//! session pair can resume conversations instead of starting fresh.

use crate::models::storage::CliSessionsFile;

use super::{run_dir, validate_id};

/// Returns the cli_sessions.json path for a run.
fn cli_sessions_path(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<std::path::PathBuf, String> {
    Ok(run_dir(workspace_path, session_id, run_id)?.join("cli_sessions.json"))
}

/// Reads the CLI sessions file for a run. Returns default if file doesn't exist.
pub fn read_cli_sessions(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
) -> Result<CliSessionsFile, String> {
    validate_id(run_id)?;
    let path = cli_sessions_path(workspace_path, session_id, run_id)?;

    if !path.exists() {
        return Ok(CliSessionsFile::default());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read cli_sessions.json: {e}"))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse cli_sessions.json: {e}"))
}

/// Writes the CLI sessions file atomically.
pub fn write_cli_sessions(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    file: &CliSessionsFile,
) -> Result<(), String> {
    validate_id(run_id)?;
    let path = cli_sessions_path(workspace_path, session_id, run_id)?;

    let json = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Failed to serialise cli_sessions: {e}"))?;
    crate::storage::atomic_write(&path, &json)
}

/// Updates a single session pair entry, creating the file if needed.
pub fn update_cli_session(
    workspace_path: &str,
    session_id: &str,
    run_id: &str,
    pair_name: &str,
    entry: crate::models::storage::CliSessionEntry,
) -> Result<(), String> {
    let mut file = read_cli_sessions(workspace_path, session_id, run_id)?;
    file.sessions.insert(pair_name.to_string(), entry);
    write_cli_sessions(workspace_path, session_id, run_id, &file)
}
