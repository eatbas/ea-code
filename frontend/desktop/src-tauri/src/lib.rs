mod agents;
mod commands;
mod events;
mod git;
mod models;
mod orchestrator;
pub mod sidecar;
pub mod storage;

use commands::AppState;
use sidecar::SidecarManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Migrate config directory from old location (AppData/Roaming/ea-code/) to new (~/.ea-code/)
    if let Err(e) = storage::recovery::migrate_config_dir() {
        eprintln!("Warning: config directory migration failed: {e}");
    }

    // Migrate flat sessions/ layout to projects/{id}/sessions/{id}/ hierarchy
    if let Err(e) = storage::migration::migrate_to_project_hierarchy() {
        eprintln!("Warning: storage hierarchy migration failed: {e}");
    }

    // Ensure storage directories exist
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

    // Run crash recovery to mark any interrupted runs as crashed
    if let Err(e) = storage::recovery::recover_all_crashed() {
        eprintln!("Warning: crash recovery failed: {e}");
    }

    // Retention cleanup: delete completed runs older than configured threshold
    if let Ok(settings) = storage::settings::read_settings() {
        if settings.retention_days > 0 {
            match storage::cleanup::cleanup_old_runs(settings.retention_days) {
                Ok(()) => {}
                Err(e) => eprintln!("Warning: retention cleanup failed: {e}"),
            }
        }
    }

    // Startup cleanup: remove stale temp files, dead MCP configs, legacy SQLite, etc.
    if let Err(e) = storage::cleanup::cleanup_stale_temp_files() {
        eprintln!("Warning: startup temp file cleanup failed: {e}");
    }

    // Remove projects whose workspace folder no longer exists on disk
    if let Err(e) = storage::projects::cleanup_missing_projects() {
        eprintln!("Warning: stale project cleanup failed: {e}");
    }

    // Sync built-in MCP catalog
    if let Err(e) = storage::mcp::sync_builtin_catalog() {
        eprintln!("Warning: MCP catalog sync failed: {e}");
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
            // Create a dummy manager pointing at a non-existent dir;
            // ensure_running() will fail gracefully at pipeline time.
            SidecarManager::new(std::path::PathBuf::from("hive-api"), hive_api_port)
        }
    };

    let app = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            pause_flags: Arc::new(Mutex::new(HashMap::new())),
            answer_senders: Arc::new(Mutex::new(HashMap::new())),
            sidecar: sidecar.clone(),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::has_live_sessions,
            // Workspace commands
            commands::workspace::select_workspace,
            commands::workspace::validate_environment,
            commands::workspace::check_prerequisites,
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
            commands::mcp::get_mcp_config,
            commands::mcp::save_mcp_config,
            commands::mcp::sync_mcp_catalog,
            commands::mcp::set_context7_api_key,
            commands::mcp_runtime::get_mcp_cli_runtime_statuses,
            commands::mcp_runtime::run_cli_mcp_fix_with_prompt,
            // CLI health & version commands (legacy — to be removed)
            commands::cli::check_cli_health,
            commands::cli::get_cli_versions,
            commands::cli::update_cli,
            commands::cli::invalidate_cli_cache,
            // hive-api health & provider commands
            commands::api_health::check_api_health,
            commands::api_health::get_api_providers,
            commands::api_health::get_api_cli_versions,
            commands::api_health::update_api_cli,
            // History / session commands
            commands::history::list_projects,
            commands::history::list_sessions,
            commands::history::get_session_detail,
            commands::history::create_session,
            commands::history::get_run_detail,
            commands::history::get_run_events,
            commands::history::get_run_artifacts,
            commands::history::get_session_messages,
            commands::history::delete_session,
            commands::history::delete_project,
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
