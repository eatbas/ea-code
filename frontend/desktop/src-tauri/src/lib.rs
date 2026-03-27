mod bootstrap;
mod commands;
mod git;
mod models;
pub mod platform;
pub mod sidecar;
pub mod storage;

use bootstrap::{build_sidecar, run_startup_maintenance, spawn_sidecar_startup, stop_sidecar};
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_startup_maintenance();

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

    let sidecar = build_sidecar();

    let app = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {})
        .invoke_handler(tauri::generate_handler![
            commands::workspace::select_workspace,
            commands::workspace::validate_environment,
            commands::workspace::check_prerequisites,
            commands::workspace::list_projects,
            commands::workspace::delete_project,
            commands::workspace::open_project_folder,
            commands::workspace::open_in_vscode,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::cli::health::check_cli_health,
            commands::cli::health::get_cli_versions,
            commands::cli::health::update_cli,
            commands::cli::availability::invalidate_cli_cache,
            commands::api_health::check_api_health,
            commands::api_health::get_api_providers,
            commands::api_health::get_api_cli_versions,
            commands::api_health::update_api_cli,
        ])
        .build(tauri::generate_context!())
        .expect("error whilst building tauri application");

    spawn_sidecar_startup(sidecar.clone());

    app.run(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            stop_sidecar(&sidecar);
        }
    });
}
