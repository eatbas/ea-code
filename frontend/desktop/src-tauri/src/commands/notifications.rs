/// OS-level notification commands backed by tauri-plugin-notification (for
/// permissions) and notify-rust (for sending, so we control the app icon).

use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

/// The bundle identifier used by macOS to resolve the app icon.
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
/// Uses `notify-rust` directly so we can set the macOS application identifier
/// to our bundle ID in both dev and production builds, ensuring the Maestro
/// icon appears instead of Terminal's.
#[tauri::command]
pub fn send_notification(_app: AppHandle, title: String, body: String) -> Result<(), String> {
    // On macOS, tell the notification system our bundle ID so the correct
    // app icon is shown — even during development.
    #[cfg(target_os = "macos")]
    {
        let _ = notify_rust::set_application(BUNDLE_ID);
    }

    let mut notification = notify_rust::Notification::new();
    notification.summary(&title).body(&body);

    // On Windows, point to the bundled icon explicitly.
    #[cfg(windows)]
    {
        use tauri::Manager;
        if let Ok(resource_dir) = _app.path().resource_dir() {
            let icon_path = resource_dir.join("icons").join("icon.png");
            if icon_path.exists() {
                notification.icon(&icon_path.to_string_lossy());
            }
        }
        notification.app_id(BUNDLE_ID);
    }

    // On Linux, auto-detect the icon from the desktop entry.
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        notification.auto_icon();
    }

    tauri::async_runtime::spawn(async move {
        let _ = notification.show();
    });

    Ok(())
}
