use std::path::Path;
use std::time::Duration;

use tokio::process::{Child, Command};

pub(crate) const DEFAULT_PORT: u16 = 8719;
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestartState {
    Missing,
    Running,
    Exited,
    Unknown,
}

pub(crate) fn normalise_port(port: u16) -> u16 {
    if port == 0 {
        DEFAULT_PORT
    } else {
        port
    }
}

pub(crate) fn build_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

pub(crate) async fn spawn_symphony_process(
    venv_python: &Path,
    symphony_dir: &Path,
    port: u16,
) -> Result<Child, String> {
    let port_str = port.to_string();
    let config_path = symphony_dir.join("config.toml");

    let mut command = Command::new(venv_python);
    command
        .args([
            "-m",
            "uvicorn",
            "symphony.main:app",
            "--host",
            "127.0.0.1",
            "--port",
            &port_str,
        ])
        .env("SYMPHONY_CONFIG", &config_path)
        .current_dir(symphony_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    #[cfg(unix)]
    command.process_group(0);

    command
        .spawn()
        .map_err(|error| format!("Failed to start symphony: {error}"))
}

pub(crate) fn inspect_child_state(child: &mut Child) -> RestartState {
    match child.try_wait() {
        Ok(Some(_)) => RestartState::Exited,
        Ok(None) => RestartState::Running,
        Err(_) => RestartState::Unknown,
    }
}

pub(crate) fn requires_restart(state: RestartState) -> bool {
    !matches!(state, RestartState::Running)
}

pub(crate) async fn stop_symphony_process(child: &mut Child) {
    #[cfg(target_os = "windows")]
    if let Some(pid) = child.id() {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
    }

    #[cfg(not(target_os = "windows"))]
    if let Some(pid) = child.id() {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGTERM);
        }
    }

    match tokio::time::timeout(SHUTDOWN_GRACE, child.wait()).await {
        Ok(Ok(status)) => {
            eprintln!("[sidecar] symphony exited with status: {status}");
        }
        Ok(Err(error)) => {
            eprintln!("[sidecar] Error waiting for symphony exit: {error}");
        }
        Err(_) => {
            eprintln!("[sidecar] symphony did not exit in time — force killing");
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

#[cfg(test)]
mod tests {
    use super::{requires_restart, RestartState};

    #[test]
    fn restart_is_required_for_missing_or_terminated_processes() {
        assert!(requires_restart(RestartState::Missing));
        assert!(requires_restart(RestartState::Exited));
        assert!(requires_restart(RestartState::Unknown));
        assert!(!requires_restart(RestartState::Running));
    }
}
