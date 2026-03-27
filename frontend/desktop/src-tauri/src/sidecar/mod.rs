mod cleanup;
mod discovery;
mod health;
mod process;
pub mod python;
mod setup;

pub use discovery::find_hive_dir;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::process::Child;
use tokio::sync::Mutex;

use cleanup::kill_orphaned_hive_api;
use process::{
    build_base_url,
    inspect_child_state,
    normalise_port,
    requires_restart,
    spawn_hive_api_process,
    stop_hive_api_process,
    RestartState,
};
use setup::prepare_hive_environment;

/// Manages the hive-api sidecar process lifecycle.
#[derive(Clone)]
pub struct SidecarManager {
    inner: Arc<Mutex<SidecarInner>>,
}

struct SidecarInner {
    child: Option<Child>,
    port: u16,
    hive_dir: PathBuf,
    setup_complete: bool,
}

impl SidecarManager {
    /// Create a new manager. Call `start()` to actually launch the process.
    pub fn new(hive_dir: PathBuf, port: u16) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SidecarInner {
                child: None,
                port: normalise_port(port),
                hive_dir,
                setup_complete: false,
            })),
        }
    }

    /// The base URL for API calls (e.g. "http://127.0.0.1:8719").
    pub async fn base_url(&self) -> String {
        let inner = self.inner.lock().await;
        build_base_url(inner.port)
    }

    /// Start the sidecar: clean up stale processes, ensure env, and launch uvicorn.
    pub async fn start(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;

        if inner.child.is_some() {
            return Ok(());
        }

        kill_orphaned_hive_api(inner.port).await;

        let prepared = prepare_hive_environment(&inner.hive_dir, inner.setup_complete).await?;
        let child = spawn_hive_api_process(&prepared.venv_python, &inner.hive_dir, inner.port).await?;

        eprintln!("[sidecar] hive-api process spawned on port {}", inner.port);
        inner.setup_complete = prepared.setup_complete;
        inner.child = Some(child);

        Ok(())
    }

    /// Poll `/health` until the API is ready.
    pub async fn wait_until_healthy(&self) -> Result<(), String> {
        health::wait_for_health(&self.base_url().await).await
    }

    /// Check if the API is reachable right now.
    pub async fn is_healthy(&self) -> bool {
        health::is_healthy(&self.base_url().await).await
    }

    /// Ensure the sidecar is running. If it crashed, restart it.
    pub async fn ensure_running(&self) -> Result<(), String> {
        let needs_restart = {
            let mut inner = self.inner.lock().await;
            match &mut inner.child {
                Some(child) => {
                    let state = inspect_child_state(child);
                    match state {
                        RestartState::Exited => {
                            eprintln!("[sidecar] hive-api exited unexpectedly — restarting");
                            inner.child = None;
                        }
                        RestartState::Unknown => {
                            eprintln!("[sidecar] Failed to check hive-api status — restarting");
                            inner.child = None;
                        }
                        RestartState::Missing | RestartState::Running => {}
                    }
                    requires_restart(state)
                }
                None => requires_restart(RestartState::Missing),
            }
        };

        if needs_restart {
            self.start().await?;
            self.wait_until_healthy().await?;
        }

        Ok(())
    }

    /// Gracefully stop the sidecar process.
    pub async fn stop(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if let Some(mut child) = inner.child.take() {
            eprintln!("[sidecar] Stopping hive-api…");
            stop_hive_api_process(&mut child).await;
        }
        Ok(())
    }
}
