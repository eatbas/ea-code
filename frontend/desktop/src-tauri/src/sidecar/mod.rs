mod discovery;
pub mod python;

pub use discovery::find_hive_dir;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use python::{find_python, venv_is_valid, venv_python};

const DEFAULT_PORT: u16 = 8719;
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(500);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

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
                port: if port == 0 { DEFAULT_PORT } else { port },
                hive_dir,
                setup_complete: false,
            })),
        }
    }

    /// The base URL for API calls (e.g. "http://127.0.0.1:8719").
    pub async fn base_url(&self) -> String {
        let inner = self.inner.lock().await;
        format!("http://127.0.0.1:{}", inner.port)
    }

    /// Start the sidecar: find Python, ensure venv, install deps, launch uvicorn.
    pub async fn start(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;

        if inner.child.is_some() {
            return Ok(()); // already running
        }

        // Kill any orphaned hive-api processes from a previous session.
        kill_orphaned_hive_api(inner.port).await;

        // 1. Find Python
        let python = find_python().await?;

        // 2. Ensure venv
        let venv_dir = inner.hive_dir.join(".venv");
        if !venv_is_valid(&venv_dir).await {
            eprintln!("[sidecar] Creating hive-api virtual environment…");
            let status = python
                .venv_command(&venv_dir)
                .status()
                .await
                .map_err(|e| format!("Failed to create venv: {e}"))?;
            if !status.success() {
                return Err("Failed to create hive-api virtual environment".into());
            }
        }

        // 3. Install dependencies if needed
        let venv_py = venv_python(&venv_dir);
        if !inner.setup_complete {
            // Pre-flight: verify source code exists before attempting install.
            if !inner.hive_dir.join("pyproject.toml").exists() {
                return Err(format!(
                    "hive-api source not found at {}. Run `git submodule update --init`.",
                    inner.hive_dir.display()
                ));
            }

            eprintln!("[sidecar] Installing hive-api dependencies…");
            let _ = Command::new(&venv_py)
                .args(["-m", "pip", "install", "--quiet", "--upgrade", "pip"])
                .current_dir(&inner.hive_dir)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
                .await;

            let output = Command::new(&venv_py)
                .args(["-m", "pip", "install", "--quiet", "-e", "."])
                .current_dir(&inner.hive_dir)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Dependency install failed: {e}"))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to install hive-api dependencies: {stderr}"));
            }
            inner.setup_complete = true;
        }

        // 4. Launch uvicorn
        let port_str = inner.port.to_string();
        let config_path = inner.hive_dir.join("config.toml");

        let mut cmd = Command::new(&venv_py);
        cmd.args([
            "-m",
            "uvicorn",
            "hive_api.main:app",
            "--host",
            "127.0.0.1",
            "--port",
            &port_str,
        ])
        .env("HIVE_API_CONFIG", &config_path)
        .current_dir(&inner.hive_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

        // Create a new process group so stop() can kill the entire tree.
        #[cfg(unix)]
        cmd.process_group(0);

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start hive-api: {e}"))?;

        eprintln!("[sidecar] hive-api process spawned on port {}", inner.port);
        inner.child = Some(child);

        Ok(())
    }

    /// Poll `/health` until the API is ready, up to `HEALTH_TIMEOUT`.
    pub async fn wait_until_healthy(&self) -> Result<(), String> {
        let url = format!("{}/health", self.base_url().await);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| format!("HTTP client error: {e}"))?;

        let deadline = tokio::time::Instant::now() + HEALTH_TIMEOUT;
        loop {
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    eprintln!("[sidecar] hive-api is healthy");
                    return Ok(());
                }
                _ => {}
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "hive-api did not become healthy within {}s",
                    HEALTH_TIMEOUT.as_secs()
                ));
            }
            tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
        }
    }

    /// Check if the API is reachable right now.
    pub async fn is_healthy(&self) -> bool {
        let url = format!("{}/health", self.base_url().await);
        let Ok(client) = reqwest::Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
        else {
            return false;
        };
        client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Ensure the sidecar is running. If it crashed, restart it.
    pub async fn ensure_running(&self) -> Result<(), String> {
        let needs_restart = {
            let mut inner = self.inner.lock().await;
            match &mut inner.child {
                Some(child) => match child.try_wait() {
                    Ok(Some(_exit)) => {
                        eprintln!("[sidecar] hive-api exited unexpectedly — restarting");
                        inner.child = None;
                        true
                    }
                    Ok(None) => false, // still running
                    Err(e) => {
                        eprintln!("[sidecar] Failed to check hive-api status: {e} — restarting");
                        inner.child = None;
                        true
                    }
                },
                None => true, // never started
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

            // On Windows, use taskkill to kill the entire process tree
            #[cfg(target_os = "windows")]
            if let Some(pid) = child.id() {
                let _ = Command::new("taskkill")
                    .args(["/T", "/F", "/PID", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await;
            }

            // On Unix/macOS, kill the entire process group so worker
            // processes are terminated alongside the parent.
            #[cfg(not(target_os = "windows"))]
            if let Some(pid) = child.id() {
                unsafe {
                    // SIGTERM the process group (negative PID).
                    libc::kill(-(pid as i32), libc::SIGTERM);
                }
            }

            // Wait with a grace period, then force kill if needed
            match tokio::time::timeout(SHUTDOWN_GRACE, child.wait()).await {
                Ok(Ok(status)) => {
                    eprintln!("[sidecar] hive-api exited with status: {status}");
                }
                Ok(Err(e)) => {
                    eprintln!("[sidecar] Error waiting for hive-api exit: {e}");
                }
                Err(_) => {
                    eprintln!("[sidecar] hive-api did not exit in time — force killing");
                    #[cfg(not(target_os = "windows"))]
                    if let Some(pid) = child.id() {
                        unsafe {
                            libc::kill(-(pid as i32), libc::SIGKILL);
                        }
                    }
                    let _ = child.kill().await;
                }
            }
        }
        Ok(())
    }
}

/// Kill any orphaned hive-api uvicorn processes listening on the given port.
///
/// This handles the case where a previous app session crashed or was force-quit
/// without properly stopping the sidecar.
async fn kill_orphaned_hive_api(port: u16) {
    let port_str = port.to_string();

    #[cfg(not(target_os = "windows"))]
    {
        // Use lsof to find processes listening on the port, then SIGKILL them.
        let output = Command::new("lsof")
            .args(["-ti", &format!("tcp:{port_str}")])
            .output()
            .await;

        if let Ok(output) = output {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.split_whitespace() {
                if let Ok(pid) = pid_str.parse::<i32>() {
                    eprintln!("[sidecar] Killing orphaned process {pid} on port {port_str}");
                    unsafe {
                        libc::kill(pid, libc::SIGKILL);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Use netstat to find PIDs, then taskkill to terminate the process tree.
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{port_str} | findstr LISTENING")])
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut killed: std::collections::HashSet<String> = std::collections::HashSet::new();
            for line in stdout.lines() {
                // netstat output: TCP  127.0.0.1:8719  0.0.0.0:0  LISTENING  12345
                if let Some(pid_str) = line.split_whitespace().last() {
                    if killed.contains(pid_str) {
                        continue;
                    }
                    eprintln!(
                        "[sidecar] Killing orphaned process {pid_str} on port {port_str}"
                    );
                    let _ = Command::new("taskkill")
                        .args(["/T", "/F", "/PID", pid_str])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .await;
                    killed.insert(pid_str.to_string());
                }
            }
        }
    }
}

