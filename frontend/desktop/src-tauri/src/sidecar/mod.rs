mod cleanup;
mod discovery;
mod health;
pub(crate) mod log_buffer;
mod process;
pub mod python;
mod setup;

pub use discovery::find_symphony_dir;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::AppHandle;
use tokio::process::Child;
use tokio::sync::Mutex;

use cleanup::kill_orphaned_symphony;
use log_buffer::SidecarLogBuffer;
use process::{
    build_base_url, inspect_child_state, normalise_port, requires_restart, spawn_symphony_process,
    stop_symphony_process, RestartState,
};
use setup::prepare_symphony_environment;

/// Manages the Symphony sidecar process lifecycle.
#[derive(Clone)]
pub struct SidecarManager {
    inner: Arc<Mutex<SidecarInner>>,
}

struct SidecarInner {
    child: Option<Child>,
    port: u16,
    symphony_dir: PathBuf,
    setup_complete: bool,
    app: Option<AppHandle>,
    log_buffer: SidecarLogBuffer,
}

impl SidecarManager {
    /// Create a new manager. Call `start()` to actually launch the process.
    pub fn new(symphony_dir: PathBuf, port: u16) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SidecarInner {
                child: None,
                port: normalise_port(port),
                symphony_dir,
                setup_complete: false,
                app: None,
                log_buffer: SidecarLogBuffer::new(),
            })),
        }
    }

    /// Inject the Tauri app handle so log events can be emitted to the frontend.
    pub async fn set_app_handle(&self, app: AppHandle) {
        self.inner.lock().await.app = Some(app);
    }

    /// Return a clone of the log buffer for retroactive retrieval.
    pub async fn log_buffer(&self) -> SidecarLogBuffer {
        self.inner.lock().await.log_buffer.clone()
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

        kill_orphaned_symphony(inner.port).await;

        let prepared =
            prepare_symphony_environment(&inner.symphony_dir, inner.setup_complete).await?;
        let child = spawn_symphony_process(
            &prepared.venv_python,
            &inner.symphony_dir,
            inner.port,
            inner.app.clone(),
            Some(inner.log_buffer.clone()),
        )
        .await?;

        eprintln!("[sidecar] symphony process spawned on port {}", inner.port);
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
                            eprintln!("[sidecar] symphony exited unexpectedly — restarting");
                            inner.child = None;
                        }
                        RestartState::Unknown => {
                            eprintln!("[sidecar] Failed to check symphony status — restarting");
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
            eprintln!("[sidecar] Stopping symphony…");
            stop_symphony_process(&mut child).await;
        }
        Ok(())
    }
}
