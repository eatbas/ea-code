use crate::storage;

/// Returns whether any persisted session is still live anywhere in the app.
#[tauri::command]
pub async fn has_live_sessions() -> Result<bool, String> {
    let sessions = storage::sessions::list_all_sessions()?;

    Ok(sessions.into_iter().any(|session| {
        matches!(
            session.last_status.as_deref(),
            Some("running" | "paused" | "waiting_for_input")
        )
    }))
}
