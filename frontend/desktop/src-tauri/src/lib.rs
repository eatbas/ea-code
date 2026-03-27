mod commands;
mod git;
mod models;
pub mod storage;

use commands::AppState;

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

    app.run(|_app_handle, _event| {});
}
