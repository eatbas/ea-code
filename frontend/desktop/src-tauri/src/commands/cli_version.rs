//! CLI version detection and update helpers.
//!
//! Extracted from `cli.rs` to keep file sizes under 300 lines.

use crate::models::CliVersionInfo;

use super::cli::check_binary_exists;
use super::cli_util::extract_version_from_output;
#[cfg(target_os = "windows")]
use super::git_bash;
#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};

pub(crate) async fn get_installed_version(path: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = git_bash::run_binary(path, &["--version"], 5).await?;
        if !output.status.success() {
            return None;
        }
        extract_version_from_output(&output)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = tokio::process::Command::new(path);
        cmd.arg("--version").kill_on_drop(true);
        let output = timeout(Duration::from_secs(5), cmd.output())
            .await
            .ok()?
            .ok()?;
        if output.status.success() {
            return extract_version_from_output(&output);
        }
        None
    }
}

/// Reads the installed version from the npm package.json on disk (no process spawn).
/// Fallback for CLIs whose `--version` hangs (e.g. gemini in non-TTY contexts).
async fn get_npm_package_version(npm_package: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        let path = format!("{appdata}\\npm\\node_modules\\{npm_package}\\package.json");
        let content = tokio::fs::read_to_string(&path).await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        return json["version"].as_str().map(|s| s.to_string());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").ok()?;
        for prefix in [
            "/usr/local/lib".to_string(),
            "/usr/lib".to_string(),
            format!("{home}/.local/lib"),
        ] {
            let path = format!("{prefix}/node_modules/{npm_package}/package.json");
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(v) = json["version"].as_str() {
                        return Some(v.to_string());
                    }
                }
            }
        }
        None
    }
}

/// Fetches the latest version via HTTP (no process spawns).
async fn get_latest_version(cli_name: &str, npm_package: &str) -> Option<String> {
    if cli_name == "kimi" {
        if let Some(v) = super::cli_http::get_latest_pypi_version("kimi-cli").await {
            return Some(v);
        }
        return super::cli_http::get_latest_npm_version_http(npm_package).await;
    }
    super::cli_http::get_latest_npm_version_http(npm_package).await
}

pub(super) async fn build_cli_version_info(
    path: &str,
    display_name: &str,
    cli_name: &str,
    npm_package: &str,
) -> CliVersionInfo {
    let update_command = if cli_name == "kimi" {
        "uv tool upgrade kimi-cli --no-cache".to_string()
    } else {
        format!("npm install -g {npm_package}@latest")
    };

    // Quick availability check (cache hit after check_cli_health populates it).
    let exists = check_binary_exists(path).await;
    if !exists {
        return CliVersionInfo {
            name: display_name.to_string(),
            cli_name: cli_name.to_string(),
            installed_version: None,
            latest_version: None,
            up_to_date: false,
            update_command,
            available: false,
            error: Some(format!("{path} not found in PATH")),
        };
    }

    // Phase 1: Try reading version from package.json on disk (no process spawn).
    let pkg_version = if cli_name != "kimi" {
        get_npm_package_version(npm_package).await
    } else {
        None
    };

    // Phase 2: Parallel — HTTP latest + maybe --version.
    // Skip the costly --version spawn when we already have a file-based version.
    let need_cli_version = pkg_version.is_none();
    let (cli_version, latest) = tokio::join!(
        async {
            if need_cli_version {
                get_installed_version(path).await
            } else {
                None
            }
        },
        get_latest_version(cli_name, npm_package),
    );
    let installed = pkg_version.or(cli_version);
    let up_to_date = matches!((&installed, &latest), (Some(i), Some(l)) if i == l);
    let error = match (&installed, &latest) {
        (None, None) => Some(format!("Failed to read version info for {path}")),
        (None, Some(_)) => Some(format!(
            "Failed to read installed version from {path} --version"
        )),
        (Some(_), None) => Some(format!("Failed to fetch latest version for {npm_package}")),
        (Some(_), Some(_)) => None,
    };
    CliVersionInfo {
        name: display_name.to_string(),
        cli_name: cli_name.to_string(),
        installed_version: installed,
        latest_version: latest,
        up_to_date,
        update_command,
        available: true,
        error,
    }
}

pub(super) async fn build_git_bash_version_info() -> CliVersionInfo {
    let exists = check_binary_exists("bash").await;
    if !exists {
        return CliVersionInfo {
            name: "Git Bash CLI".to_string(),
            cli_name: "gitBash".to_string(),
            installed_version: None,
            latest_version: None,
            up_to_date: false,
            update_command: "https://git-scm.com/download/win".to_string(),
            available: false,
            error: Some("Git Bash is required on Windows to run agents".to_string()),
        };
    }

    let (installed, latest) = tokio::join!(
        get_installed_version("git"),
        super::cli_http::get_latest_git_version_http(),
    );
    let up_to_date =
        matches!((&installed, &latest), (Some(i), Some(l)) if i == l || i.starts_with(l));
    let error = match (&installed, &latest) {
        (None, None) => {
            Some("Failed to read installed and latest version for Git Bash".to_string())
        }
        (None, Some(_)) => Some("Failed to read installed version from git --version".to_string()),
        (Some(_), None) => Some("Failed to fetch latest version for Git Bash".to_string()),
        (Some(_), Some(_)) => None,
    };
    CliVersionInfo {
        name: "Git Bash CLI".to_string(),
        cli_name: "gitBash".to_string(),
        installed_version: installed,
        latest_version: latest,
        up_to_date,
        update_command: "https://git-scm.com/download/win".to_string(),
        available: true,
        error,
    }
}
