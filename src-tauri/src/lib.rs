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
    let pool = db::init_db().unwrap_or_else(|e| {
        panic!("Failed to initialise database — check permissions for ~/.config/ea-code/: {e}");
    });

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
            // Workspace commands
            commands::workspace::select_workspace,
            commands::workspace::validate_environment,
            commands::workspace::open_in_vscode,
            // Pipeline commands
            commands::pipeline::run_pipeline,
            commands::pipeline::cancel_pipeline,
            commands::pipeline::answer_pipeline_question,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_project_settings,
            commands::settings::save_project_settings,
            commands::settings::clear_project_settings,
            // Skills commands
            commands::skills::list_skills,
            commands::skills::get_skill,
            commands::skills::create_skill,
            commands::skills::update_skill,
            commands::skills::delete_skill,
            // MCP server commands
            commands::mcp::list_mcp_servers,
            commands::mcp::list_mcp_capable_clis,
            commands::mcp::set_mcp_server_enabled,
            commands::mcp::set_mcp_server_bindings,
            commands::mcp::create_mcp_server,
            commands::mcp::update_mcp_server,
            commands::mcp::delete_mcp_server,
            commands::mcp::set_context7_api_key,
            // CLI health & version commands
            commands::cli::check_cli_health,
            commands::cli::get_cli_versions,
            commands::cli::update_cli,
            // History / session commands
            commands::history::list_projects,
            commands::history::list_sessions,
            commands::history::get_session_detail,
            commands::history::create_session,
            commands::history::get_run_detail,
            commands::history::get_run_logs,
            commands::history::get_run_artifacts,
            commands::history::delete_session,
        ])
        .run(tauri::generate_context!())
        .expect("error whilst running tauri application");
}
