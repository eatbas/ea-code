use std::path::{Path, PathBuf};

use tokio::process::Command;

use super::python::{find_python, venv_is_valid, venv_python};

pub(crate) struct PreparedEnvironment {
    pub venv_python: PathBuf,
    pub setup_complete: bool,
}

pub(crate) async fn prepare_hive_environment(
    hive_dir: &Path,
    setup_complete: bool,
) -> Result<PreparedEnvironment, String> {
    let python = find_python().await?;
    let venv_dir = hive_dir.join(".venv");

    if !venv_is_valid(&venv_dir).await {
        eprintln!("[sidecar] Creating hive-api virtual environment…");
        let status = python
            .venv_command(&venv_dir)
            .status()
            .await
            .map_err(|error| format!("Failed to create venv: {error}"))?;
        if !status.success() {
            return Err("Failed to create hive-api virtual environment".into());
        }
    }

    let venv_python = venv_python(&venv_dir);
    let setup_complete = ensure_dependencies(&venv_python, hive_dir, setup_complete).await?;

    Ok(PreparedEnvironment {
        venv_python,
        setup_complete,
    })
}

async fn ensure_dependencies(
    venv_python: &Path,
    hive_dir: &Path,
    setup_complete: bool,
) -> Result<bool, String> {
    if setup_complete {
        return Ok(true);
    }

    if !hive_dir.join("pyproject.toml").exists() {
        return Err(format!(
            "hive-api source not found at {}. Run `git submodule update --init`.",
            hive_dir.display()
        ));
    }

    eprintln!("[sidecar] Installing hive-api dependencies…");
    let _ = Command::new(venv_python)
        .args(["-m", "pip", "install", "--quiet", "--upgrade", "pip"])
        .current_dir(hive_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .await;

    let output = Command::new(venv_python)
        .args(["-m", "pip", "install", "--quiet", "-e", "."])
        .current_dir(hive_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|error| format!("Dependency install failed: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to install hive-api dependencies: {stderr}"));
    }

    Ok(true)
}
