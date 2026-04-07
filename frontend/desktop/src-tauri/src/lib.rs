mod bootstrap;
mod commands;
mod conversations;
mod git;
pub mod http;
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
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            sidecar: sidecar.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace::select_workspace,
            commands::workspace::validate_environment,
            commands::workspace::check_prerequisites,
            commands::workspace::list_projects,
            commands::workspace::delete_project,
            commands::workspace::rename_project,
            commands::workspace::archive_project,
            commands::workspace::unarchive_project,
            commands::workspace::reorder_projects,
            commands::workspace::open_project_folder,
            commands::workspace::open_in_vscode,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::cli::health::check_cli_health,
            commands::cli::health::get_cli_versions,
            commands::cli::health::update_cli,
            commands::cli::availability::invalidate_cli_cache,
            commands::api_health::check_sidecar_ready,
            commands::api_health::check_api_health,
            commands::api_health::get_api_providers,
            commands::api_health::get_api_cli_versions,
            commands::api_health::update_api_cli,
            conversations::commands::conversation_handlers::list_workspace_conversations,
            conversations::commands::conversation_handlers::create_conversation,
            conversations::commands::conversation_handlers::get_conversation,
            conversations::commands::conversation_handlers::send_conversation_turn,
            conversations::commands::conversation_handlers::stop_conversation,
            conversations::commands::conversation_handlers::delete_conversation,
            conversations::commands::conversation_handlers::rename_conversation,
            conversations::commands::conversation_handlers::archive_conversation,
            conversations::commands::conversation_handlers::unarchive_conversation,
            conversations::commands::conversation_handlers::set_conversation_pinned,
            conversations::commands::image_handlers::save_conversation_image,
            conversations::commands::pipeline_handlers::actions::start_pipeline,
            conversations::commands::pipeline_handlers::actions::stop_pipeline,
            conversations::commands::pipeline_handlers::resume::resume_pipeline,
            conversations::commands::pipeline_handlers::actions::get_pipeline_state,
            conversations::commands::pipeline_handlers::actions::get_pipeline_debug_log,
            conversations::commands::pipeline_handlers::actions::accept_plan,
            conversations::commands::pipeline_handlers::actions::send_plan_edit_feedback,
            conversations::commands::pipeline_handlers::redo_review::redo_review_pipeline,
            commands::power::enable_keep_awake,
            commands::power::disable_keep_awake,
            commands::notifications::request_notification_permission,
            commands::notifications::send_notification,
        ])
        .build(tauri::generate_context!())
        .expect("error whilst building tauri application");

    spawn_sidecar_startup(app.handle().clone(), sidecar.clone());

    // Auto-enable keep-awake if the user had it turned on.
    if let Ok(settings) = storage::settings::read_settings() {
        if settings.keep_awake {
            let _ = commands::power::enable_keep_awake();
        }
    }

    app.run(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            let _ = commands::power::disable_keep_awake();
            stop_sidecar(&sidecar);
        }
    });
}
