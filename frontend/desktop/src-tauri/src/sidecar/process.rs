use std::path::Path;
use std::time::Duration;

use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};

use super::log_buffer::{emit_sidecar_log, SidecarLogBuffer};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub(crate) const DEFAULT_PORT: u16 = 8719;
const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

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
    app: Option<AppHandle>,
    buffer: Option<SidecarLogBuffer>,
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
            "--no-access-log",
        ])
        .env("SYMPHONY_CONFIG", &config_path)
        .current_dir(symphony_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    #[cfg(unix)]
    command.process_group(0);

    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to start symphony: {error}"))?;

    if let Some(stdout) = child.stdout.take() {
        spawn_pipe_drain(stdout, "stdout", app.clone(), buffer.clone());
    }

    if let Some(stderr) = child.stderr.take() {
        spawn_pipe_drain(stderr, "stderr", app, buffer);
    }

    Ok(child)
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
            .creation_flags(CREATE_NO_WINDOW)
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

fn spawn_pipe_drain<R>(
    reader: R,
    stream_name: &'static str,
    app: Option<AppHandle>,
    buffer: Option<SidecarLogBuffer>,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if !line.trim().is_empty() {
                        eprintln!("[sidecar:{stream_name}] {line}");
                        emit_sidecar_log(app.as_ref(), buffer.as_ref(), stream_name, line);
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    eprintln!("[sidecar:{stream_name}] failed to read process output: {error}");
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::process::Stdio;
    use std::time::Duration;

    use tokio::process::Command;
    use tokio::time::timeout;

    use super::{requires_restart, RestartState};

    #[test]
    fn restart_is_required_for_missing_or_terminated_processes() {
        assert!(requires_restart(RestartState::Missing));
        assert!(requires_restart(RestartState::Exited));
        assert!(requires_restart(RestartState::Unknown));
        assert!(!requires_restart(RestartState::Running));
    }

    #[tokio::test]
    async fn pipe_drain_allows_noisy_child_to_exit() {
        let mut command = noisy_python_command().await;
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .expect("test fixture should spawn a noisy child process");

        if let Some(stdout) = child.stdout.take() {
            super::spawn_pipe_drain(stdout, "test-stdout", None, None);
        }

        if let Some(stderr) = child.stderr.take() {
            super::spawn_pipe_drain(stderr, "test-stderr", None, None);
        }

        let status = timeout(Duration::from_secs(10), child.wait())
            .await
            .expect("draining sidecar pipes should prevent child-process deadlock")
            .expect("child process should exit cleanly");

        assert!(
            status.success(),
            "expected noisy child to exit successfully"
        );
    }

    async fn noisy_python_command() -> Command {
        let interpreter = crate::sidecar::python::find_python()
            .await
            .expect("Python 3.12+ is required for sidecar tests");

        let mut command = Command::new(&interpreter.executable);
        if let Some(version_flag) = interpreter.launcher_version.as_deref() {
            command.arg(version_flag);
        }

        // Emit enough blank lines to exceed a typical pipe buffer without
        // cluttering captured test output when the drain helper is active.
        command.args([
            "-c",
            "import sys; chunk = '\\n' * 200000; sys.stdout.write(chunk); sys.stdout.flush(); sys.stderr.write(chunk); sys.stderr.flush()",
        ]);
        command
    }
}
