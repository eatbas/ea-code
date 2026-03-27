mod commands;
mod git;
mod models;
pub mod sidecar;
pub mod storage;

use commands::AppState;
use sidecar::SidecarManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Ensure global storage directories exist
    if let Err(e) = storage::ensure_dirs() {
        eprintln!("Failed to initialise storage directories: {e}");
    }

    // N7: Recover any orphaned backup files from interrupted atomic writes
    if let Err(e) = storage::recover_orphaned_backups() {
        eprintln!("Warning: failed to recover orphaned backups: {e}");
    }

    // Import legacy settings from SQLite if needed (one-time migration)
    if let Err(e) = storage::settings::import_from_legacy_json() {
        eprintln!("Warning: failed to import legacy settings: {e}");
    }

    // Remove projects whose workspace folder no longer exists on disk
    if let Err(e) = storage::projects::cleanup_missing_projects() {
        eprintln!("Warning: stale project cleanup failed: {e}");
    }

    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        use tauri::Manager;

        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    // Resolve hive-api directory and read configured port from settings
    let hive_api_port = storage::settings::read_settings()
        .map(|s| s.hive_api_port)
        .unwrap_or(0);
    let sidecar = match sidecar::find_hive_dir() {
        Ok(hive_dir) => SidecarManager::new(hive_dir, hive_api_port),
        Err(e) => {
            eprintln!("Warning: {e} — hive-api sidecar will not auto-start");
            SidecarManager::new(std::path::PathBuf::from("hive-api"), hive_api_port)
        }
    };

    let app = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {})
        .invoke_handler(tauri::generate_handler![
            // Workspace commands
            commands::workspace::select_workspace,
            commands::workspace::validate_environment,
            commands::workspace::check_prerequisites,
            commands::workspace::list_projects,
            commands::workspace::delete_project,
            commands::workspace::open_in_vscode,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::save_settings,
            // CLI health & version commands
            commands::cli::check_cli_health,
            commands::cli::get_cli_versions,
            commands::cli::update_cli,
            commands::cli::invalidate_cli_cache,
            // hive-api health & provider commands
            commands::api_health::check_api_health,
            commands::api_health::get_api_providers,
            commands::api_health::get_api_cli_versions,
            commands::api_health::update_api_cli,
        ])
        .build(tauri::generate_context!())
        .expect("error whilst building tauri application");

    // Start the hive-api sidecar in the background (non-blocking)
    let sidecar_for_startup = sidecar.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = sidecar_for_startup.start().await {
            eprintln!("Warning: failed to start hive-api sidecar: {e}");
            return;
        }
        if let Err(e) = sidecar_for_startup.wait_until_healthy().await {
            eprintln!("Warning: hive-api sidecar not healthy: {e}");
        }
    });

    app.run(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            let sc = sidecar.clone();
            tauri::async_runtime::block_on(async move {
                if let Err(e) = sc.stop().await {
                    eprintln!("Warning: failed to stop hive-api sidecar: {e}");
                }
            });
        }
    });
}
