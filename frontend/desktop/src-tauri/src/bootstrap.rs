use std::path::PathBuf;

use crate::sidecar::{self, SidecarManager};
use crate::storage;

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

pub(crate) fn spawn_sidecar_startup(sidecar: SidecarManager) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = sidecar.start().await {
            eprintln!("Warning: failed to start hive-api sidecar: {error}");
            return;
        }
        if let Err(error) = sidecar.wait_until_healthy().await {
            eprintln!("Warning: hive-api sidecar not healthy: {error}");
        }
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
