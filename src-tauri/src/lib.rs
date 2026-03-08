mod agents;
mod commands;
pub mod db;
mod events;
mod git;
mod models;
mod orchestrator;
pub mod schema;

use commands::AppState;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise the SQLite database and run pending migrations
    let pool = db::init_db().expect("Failed to initialise database");

    // Import legacy settings.json if it exists (first launch only)
    if let Err(e) = db::import_legacy_settings(&pool) {
        eprintln!("Warning: failed to import legacy settings: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            answer_sender: Arc::new(Mutex::new(None)),
            db: pool,
        })
        .invoke_handler(tauri::generate_handler![
            commands::select_workspace,
            commands::validate_environment,
            commands::run_pipeline,
            commands::cancel_pipeline,
            commands::get_settings,
            commands::save_settings,
            commands::check_cli_health,
            commands::answer_pipeline_question,
            // History / session commands
            commands::list_projects,
            commands::list_sessions,
            commands::get_session_detail,
            commands::create_session,
            commands::get_run_detail,
            commands::get_run_logs,
            commands::get_run_artifacts,
            commands::delete_session,
            // CLI version management
            commands::get_cli_versions,
            commands::update_cli,
        ])
        .run(tauri::generate_context!())
        .expect("error whilst running tauri application");
}
