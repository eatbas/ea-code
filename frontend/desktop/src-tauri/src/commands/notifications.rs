/// OS-level notification commands backed by tauri-plugin-notification.
///
/// We use the Tauri plugin on every desktop platform so permission checks and
/// delivery go through the same native stack. This avoids the macOS mismatch
/// where permission requests used Tauri, but delivery used `notify-rust`.

use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

/// Request OS notification permission and return whether it was granted.
#[tauri::command]
pub fn request_notification_permission(app: AppHandle) -> Result<bool, String> {
    let state = app
        .notification()
        .request_permission()
        .map_err(|e| format!("Failed to request notification permission: {e}"))?;
    Ok(state == PermissionState::Granted)
}

/// Send an OS-level notification with the given title and body.
#[tauri::command]
pub fn send_notification(app: AppHandle, title: String, body: String) -> Result<(), String> {
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| format!("Failed to send notification: {e}"))?;

    Ok(())
}
