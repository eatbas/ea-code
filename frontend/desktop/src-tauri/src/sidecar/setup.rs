use std::path::{Path, PathBuf};

use tokio::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use super::python::{find_python, venv_is_valid, venv_python};

pub(crate) struct PreparedEnvironment {
    pub venv_python: PathBuf,
    pub setup_complete: bool,
}

pub(crate) async fn prepare_symphony_environment(
    symphony_dir: &Path,
    setup_complete: bool,
) -> Result<PreparedEnvironment, String> {
    let python = find_python().await?;
    let venv_dir = symphony_dir.join(".venv");

    if !venv_is_valid(&venv_dir).await {
        eprintln!("[sidecar] Creating symphony virtual environment…");
        let status = python
            .venv_command(&venv_dir)
            .status()
            .await
            .map_err(|error| format!("Failed to create venv: {error}"))?;
        if !status.success() {
            return Err("Failed to create symphony virtual environment".into());
        }
    }

    let venv_python = venv_python(&venv_dir);
    let setup_complete = ensure_dependencies(&venv_python, symphony_dir, setup_complete).await?;

    Ok(PreparedEnvironment {
        venv_python,
        setup_complete,
    })
}

async fn ensure_dependencies(
    venv_python: &Path,
    symphony_dir: &Path,
    setup_complete: bool,
) -> Result<bool, String> {
    if setup_complete {
        return Ok(true);
    }

    if !symphony_dir.join("pyproject.toml").exists() {
        return Err(format!(
            "symphony source not found at {}. Run `git submodule update --init`.",
            symphony_dir.display()
        ));
    }

    eprintln!("[sidecar] Installing symphony dependencies…");
    let mut pip_upgrade = Command::new(venv_python);
    pip_upgrade
        .args(["-m", "pip", "install", "--quiet", "--upgrade", "pip"])
        .current_dir(symphony_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    pip_upgrade.creation_flags(CREATE_NO_WINDOW);
    let _ = pip_upgrade.status().await;

    let mut pip_install = Command::new(venv_python);
    pip_install
        .args(["-m", "pip", "install", "--quiet", "-e", "."])
        .current_dir(symphony_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    #[cfg(target_os = "windows")]
    pip_install.creation_flags(CREATE_NO_WINDOW);
    let output = pip_install
        .output()
        .await
        .map_err(|error| format!("Dependency install failed: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to install symphony dependencies: {stderr}"));
    }

    Ok(true)
}
