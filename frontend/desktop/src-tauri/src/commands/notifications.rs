/// OS-level notification commands backed by tauri-plugin-notification (for
/// permissions and for sending on Windows/Linux) and notify-rust (for sending
/// on macOS, where we need to override the bundle ID for the correct icon).

use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

/// The bundle identifier used by macOS to resolve the app icon.
#[cfg(target_os = "macos")]
const BUNDLE_ID: &str = "com.eatbas.maestro";

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
///
/// On macOS, uses `notify-rust` directly so we can set the bundle identifier
/// to our app ID in both dev and production builds, ensuring the Maestro icon
/// appears instead of Terminal's.  On Windows and Linux, delegates to the
/// Tauri notification plugin which handles platform registration (AUMID, COM
/// threading, shortcut creation) correctly.
#[tauri::command]
pub fn send_notification(app: AppHandle, title: String, body: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = notify_rust::set_application(BUNDLE_ID);
        let mut notification = notify_rust::Notification::new();
        notification.summary(&title).body(&body);

        // show() is synchronous / blocking — run it off the async runtime.
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(e) = notification.show() {
                eprintln!("Failed to show notification: {e}");
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        app.notification()
            .builder()
            .title(&title)
            .body(&body)
            .show()
            .map_err(|e| format!("Failed to send notification: {e}"))?;
    }

    Ok(())
}
