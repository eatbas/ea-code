use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::sidecar::{self, SidecarManager};
use crate::storage;

const EVENT_SIDECAR_READY: &str = "sidecar_ready";

#[derive(Clone, Serialize)]
struct SidecarReadyPayload {
    ready: bool,
    error: Option<String>,
}

pub(crate) fn run_startup_maintenance() {
    if let Err(error) = storage::ensure_dirs() {
        eprintln!("Failed to initialise storage directories: {error}");
    }

    if let Err(error) = storage::recover_orphaned_backups() {
        eprintln!("Warning: failed to recover orphaned backups: {error}");
    }

    if let Err(error) = storage::settings::import_from_legacy_json() {
        eprintln!("Warning: failed to import legacy settings: {error}");
    }

    if let Err(error) = storage::projects::cleanup_missing_projects() {
        eprintln!("Warning: stale project cleanup failed: {error}");
    }

    match storage::projects::list_projects(true) {
        Ok(projects) => {
            let mut recovered = 0usize;
            let mut removed = 0usize;

            for project in projects {
                match crate::conversations::persistence::cleanup_orphaned_conversations(
                    &project.path,
                ) {
                    Ok(stats) => {
                        recovered += stats.recovered;
                        removed += stats.removed;
                    }
                    Err(error) => {
                        eprintln!(
                            "Warning: conversation cleanup failed for {}: {error}",
                            project.path
                        );
                    }
                }
            }

            if recovered > 0 || removed > 0 {
                eprintln!(
                    "Startup cleanup: recovered {recovered} conversation(s), removed {removed} orphaned conversation(s)"
                );
            }
        }
        Err(error) => {
            eprintln!("Warning: failed to list projects for conversation cleanup: {error}");
        }
    }
}

pub(crate) fn build_sidecar() -> SidecarManager {
    let hive_api_port = storage::settings::read_settings()
        .map(|settings| settings.hive_api_port)
        .unwrap_or(0);

    match sidecar::find_hive_dir() {
        Ok(hive_dir) => SidecarManager::new(hive_dir, hive_api_port),
        Err(error) => {
            eprintln!("Warning: {error} — hive-api sidecar will not auto-start");
            SidecarManager::new(PathBuf::from("hive-api"), hive_api_port)
        }
    }
}

pub(crate) fn spawn_sidecar_startup(app: AppHandle, sidecar: SidecarManager) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = sidecar.start().await {
            eprintln!("Warning: failed to start hive-api sidecar: {error}");
            let _ = app.emit(
                EVENT_SIDECAR_READY,
                SidecarReadyPayload {
                    ready: false,
                    error: Some(error.clone()),
                },
            );
            return;
        }
        if let Err(error) = sidecar.wait_until_healthy().await {
            eprintln!("Warning: hive-api sidecar not healthy: {error}");
            let _ = app.emit(
                EVENT_SIDECAR_READY,
                SidecarReadyPayload {
                    ready: false,
                    error: Some(error.clone()),
                },
            );
            return;
        }
        let _ = app.emit(
            EVENT_SIDECAR_READY,
            SidecarReadyPayload {
                ready: true,
                error: None,
            },
        );
    });
}

pub(crate) fn stop_sidecar(sidecar: &SidecarManager) {
    let cloned = sidecar.clone();
    tauri::async_runtime::block_on(async move {
        if let Err(error) = cloned.stop().await {
            eprintln!("Warning: failed to stop hive-api sidecar: {error}");
        }
    });
}
