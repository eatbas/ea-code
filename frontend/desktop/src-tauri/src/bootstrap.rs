#[cfg(target_os = "macos")]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::sidecar::{self, SidecarManager};
use crate::storage;

const SIDECAR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

const EVENT_SIDECAR_READY: &str = "sidecar_ready";

#[derive(Clone, Serialize)]
struct SidecarReadyPayload {
    ready: bool,
    error: Option<String>,
}

/// Fast, essential startup maintenance that runs on the main thread before the
/// Tauri builder is constructed. Keep this cheap and deterministic — anything
/// that does per-conversation I/O (and could therefore deadlock or hang on a
/// broken conversation) must be queued on the background conversation cleanup
/// task instead (see [`spawn_startup_conversation_cleanup`]).
pub(crate) fn run_startup_maintenance() {
    repair_process_environment();

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

/// Heal per-workspace conversation state in the background. Kept off the
/// synchronous startup path so a single broken conversation can never prevent
/// the main window from appearing — the previous synchronous variant was
/// observed to deadlock on a malformed conversation directory, ghosting the
/// whole app.
///
/// This runs on Tokio's blocking pool (the persistence layer uses synchronous
/// `std::sync::Mutex` locks and blocking filesystem I/O). Each project is
/// isolated with `catch_unwind` so a panic on one workspace does not abort the
/// entire pass.
pub(crate) fn spawn_startup_conversation_cleanup() {
    tauri::async_runtime::spawn(async {
        let result = tokio::task::spawn_blocking(run_conversation_cleanup_blocking).await;
        if let Err(error) = result {
            eprintln!("Warning: conversation cleanup task panicked: {error}");
        }
    });
}

fn run_conversation_cleanup_blocking() {
    let projects = match storage::projects::list_projects(true) {
        Ok(projects) => projects,
        Err(error) => {
            eprintln!("Warning: failed to list projects for conversation cleanup: {error}");
            return;
        }
    };

    let mut recovered = 0usize;
    let mut removed = 0usize;

    for project in projects {
        let project_path = project.path.clone();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::conversations::persistence::cleanup_orphaned_conversations(&project_path)
        }));

        match outcome {
            Ok(Ok(stats)) => {
                recovered += stats.recovered;
                removed += stats.removed;
            }
            Ok(Err(error)) => {
                eprintln!(
                    "Warning: conversation cleanup failed for {project_path}: {error}"
                );
            }
            Err(_) => {
                eprintln!(
                    "Warning: conversation cleanup panicked for {project_path} — skipping"
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

fn repair_process_environment() {
    #[cfg(target_os = "macos")]
    repair_macos_gui_path();
}

#[cfg(target_os = "macos")]
fn repair_macos_gui_path() {
    let shell_path = read_login_shell_path_dirs();
    let current_path = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();

    let mut merged_dirs: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for dir in shell_path
        .into_iter()
        .chain(current_path)
        .chain(well_known_macos_bin_dirs())
    {
        if dir.is_dir() && seen.insert(dir.clone()) {
            merged_dirs.push(dir);
        }
    }

    if merged_dirs.is_empty() {
        return;
    }

    match std::env::join_paths(&merged_dirs) {
        Ok(joined) => {
            let current = std::env::var_os("PATH");
            if current.as_ref() != Some(&joined) {
                std::env::set_var("PATH", &joined);
                eprintln!("[bootstrap] Repaired macOS PATH for GUI launch");
            }
        }
        Err(error) => {
            eprintln!("[bootstrap] Failed to repair macOS PATH: {error}");
        }
    }
}

#[cfg(target_os = "macos")]
fn read_login_shell_path_dirs() -> Vec<PathBuf> {
    for shell in candidate_shells() {
        let output = Command::new(&shell)
            .args(["-ilc", "printf %s \"$PATH\""])
            .output();
        let Ok(output) = output else {
            continue;
        };
        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let dirs = parse_path_dirs(stdout.as_ref());
        if !dirs.is_empty() {
            return dirs;
        }
    }

    Vec::new()
}

#[cfg(target_os = "macos")]
fn candidate_shells() -> Vec<PathBuf> {
    let mut shells: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for candidate in [
        std::env::var_os("SHELL").map(PathBuf::from),
        Some(PathBuf::from("/bin/zsh")),
        Some(PathBuf::from("/bin/bash")),
    ]
    .into_iter()
    .flatten()
    {
        if is_executable_file(&candidate) && seen.insert(candidate.clone()) {
            shells.push(candidate);
        }
    }

    shells
}

#[cfg(target_os = "macos")]
fn well_known_macos_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/opt/homebrew/sbin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/local/sbin"),
    ];

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.extend([
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            home.join(".npm-global/bin"),
            home.join("bin"),
        ]);
    }

    dirs
}

#[cfg(target_os = "macos")]
fn parse_path_dirs(raw: &str) -> Vec<PathBuf> {
    raw.trim()
        .split(':')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect()
}

#[cfg(target_os = "macos")]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

pub(crate) fn build_sidecar() -> SidecarManager {
    let symphony_port = storage::settings::read_settings()
        .map(|settings| settings.symphony_port)
        .unwrap_or(0);

    match sidecar::find_symphony_dir() {
        Ok(symphony_dir) => SidecarManager::new(symphony_dir, symphony_port),
        Err(error) => {
            eprintln!("Warning: {error} — symphony sidecar will not auto-start");
            SidecarManager::new(PathBuf::from("symphony"), symphony_port)
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use std::path::PathBuf;

    #[cfg(target_os = "macos")]
    use super::parse_path_dirs;

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_path_dirs_keeps_only_existing_directories() {
        let tmp = std::env::temp_dir();
        let raw = format!(
            "{}:/definitely/missing:{}",
            tmp.display(),
            std::env::current_dir().unwrap().display()
        );

        let dirs = parse_path_dirs(&raw);

        assert!(dirs.contains(&tmp));
        assert!(dirs.contains(&std::env::current_dir().unwrap()));
        assert!(!dirs.contains(&PathBuf::from("/definitely/missing")));
    }
}

pub(crate) fn spawn_sidecar_startup(app: AppHandle, sidecar: SidecarManager) {
    tauri::async_runtime::spawn(async move {
        sidecar.set_app_handle(app.clone()).await;

        if let Err(error) = sidecar.start().await {
            eprintln!("Warning: failed to start symphony sidecar: {error}");
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
            eprintln!("Warning: symphony sidecar not healthy: {error}");
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

        // Symphony is now reachable — reconcile any conversations that were
        // flagged running when the app last went down. This replaces the old
        // process-local HashSet check with Symphony's own truth.
        crate::conversations::reattach::run_startup_reattach_pass(app.clone()).await;
    });
}

pub(crate) fn stop_sidecar(sidecar: &SidecarManager) {
    // Block synchronously so the Python sidecar and its descendants
    // (bash shells + CLI agents on Windows) are actually killed before
    // Tauri tears down its async runtime. Fire-and-forget races with
    // shutdown: the spawned `taskkill /T /F` never runs, and Windows
    // `kill_on_drop` only terminates the direct child, leaving the
    // grandchildren orphaned.
    let cloned = sidecar.clone();
    tauri::async_runtime::block_on(async move {
        match tokio::time::timeout(SIDECAR_SHUTDOWN_TIMEOUT, cloned.stop()).await {
            Ok(Err(error)) => {
                eprintln!("Warning: failed to stop symphony sidecar: {error}");
            }
            Err(_) => {
                eprintln!(
                    "Warning: symphony sidecar shutdown exceeded {:?}",
                    SIDECAR_SHUTDOWN_TIMEOUT
                );
            }
            Ok(Ok(())) => {}
        }
    });
}
