mod agents;
mod commands;
mod events;
mod git;
mod models;
mod orchestrator;
mod settings;

use commands::AppState;
use std::sync::{atomic::AtomicBool, Arc};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            cancel_flag: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_workspace,
            commands::validate_environment,
            commands::run_pipeline,
            commands::cancel_pipeline,
            commands::get_settings,
            commands::save_settings,
            commands::check_cli_health,
        ])
        .run(tauri::generate_context!())
        .expect("error whilst running tauri application");
}
