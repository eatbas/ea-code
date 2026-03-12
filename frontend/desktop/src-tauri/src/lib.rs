mod agents;
mod commands;
pub mod db;
mod events;
mod git;
mod models;
mod orchestrator;
pub mod schema;

use commands::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
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
    let _ = db::run_status::pause_all_running(&pool);

    // Retention cleanup: delete completed runs older than configured threshold
    if let Ok(settings) = db::settings::get(&pool) {
        if settings.retention_days > 0 {
            match db::cleanup::cleanup_old_runs(&pool, settings.retention_days as i32) {
                Ok(deleted) if deleted > 0 => {
                    eprintln!("[startup] Cleaned up {deleted} old runs");
                }
                Err(e) => eprintln!("Warning: retention cleanup failed: {e}"),
                _ => {}
            }
        }
    }

    // Lightweight maintenance — let SQLite optimise its query planner stats.
    // Avoids full VACUUM which rewrites the entire file and blocks on large DBs.
    let _ = db::cleanup::pragma_optimize(&pool);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            pause_flags: Arc::new(Mutex::new(HashMap::new())),
            answer_senders: Arc::new(Mutex::new(HashMap::new())),
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
            commands::pipeline::pause_pipeline,
            commands::pipeline::resume_pipeline,
            commands::pipeline::answer_pipeline_question,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::save_settings,
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
            commands::mcp_runtime::get_mcp_cli_runtime_statuses,
            commands::mcp_runtime::run_cli_mcp_fix_with_prompt,
            // CLI health & version commands
            commands::cli::check_cli_health,
            commands::cli::get_cli_versions,
            commands::cli::update_cli,
            commands::cli::invalidate_cli_cache,
            // History / session commands
            commands::history::list_projects,
            commands::history::list_sessions,
            commands::history::get_session_detail,
            commands::history::create_session,
            commands::history::get_run_detail,
            commands::history::get_run_artifacts,
            commands::history::delete_session,
            // App settings / DB browser commands
            commands::app_settings::get_db_stats,
            commands::app_settings::get_table_rows,
            commands::app_settings::truncate_table,
            commands::app_settings::restart_app,
        ])
        .build(tauri::generate_context!())
        .expect("error whilst building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            if let Some(state) = app_handle.try_state::<AppState>() {
                let _ = db::run_status::pause_all_running(&state.db);
            }
        }
    });
}
