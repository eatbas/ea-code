use crate::models::*;
use super::cli_util::{extract_version_from_output, get_latest_npm_version, run_npm};
#[cfg(target_os = "windows")]
use super::git_bash;
#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};
#[cfg(not(target_os = "windows"))]
fn path_probe_command() -> &'static str {
    "which"
}

#[tauri::command]
pub async fn check_cli_health(settings: AppSettings) -> Result<CliHealth, String> {
    check_cli_health_inner(&settings).await
}
#[tauri::command]
pub async fn get_cli_versions(settings: AppSettings) -> Result<AllCliVersions, String> {
    let (claude, codex, gemini, kimi, opencode, git_bash) = tokio::join!(
        build_cli_version_info(&settings.claude_path, "Claude CLI", "claude", "@anthropic-ai/claude-code"),
        build_cli_version_info(&settings.codex_path, "Codex CLI", "codex", "@openai/codex",),
        build_cli_version_info(&settings.gemini_path, "Gemini CLI", "gemini", "@google/gemini-cli"),
        build_cli_version_info(&settings.kimi_path, "Kimi CLI", "kimi", "kimi-cli"),
        build_cli_version_info(&settings.opencode_path, "OpenCode CLI", "opencode", "opencode-ai"),
        async {
            if cfg!(target_os = "windows") {
                Some(build_git_bash_version_info().await)
            } else {
                None
            }
        },
    );
    Ok(AllCliVersions {
        claude,
        codex,
        gemini,
        kimi,
        opencode,
        git_bash,
    })
}
#[tauri::command]
pub async fn update_cli(cli_name: String) -> Result<String, String> {
    match cli_name.as_str() {
        "claude" => update_with_npm("@anthropic-ai/claude-code").await,
        "codex" => update_with_npm("@openai/codex").await,
        "gemini" => update_with_npm("@google/gemini-cli").await,
        "opencode" => update_with_npm("opencode-ai").await,
        "kimi" => update_kimi_cli().await,
        _ => Err(format!("Unknown CLI: {cli_name}")),
    }
}
async fn check_single_cli(path: &str) -> CliStatus {
    #[cfg(target_os = "windows")]
    {
        let available = git_bash::command_exists(path).await;
        return if available {
            CliStatus { available: true, path: path.to_string(), error: None }
        } else {
            CliStatus { available: false, path: path.to_string(), error: Some(format!("{path} not found in Git Bash PATH")) }
        };
    }
    #[cfg(not(target_os = "windows"))]
    let probe = path_probe_command();
    #[cfg(not(target_os = "windows"))]
    match tokio::process::Command::new(probe).arg(path).output().await {
        Ok(output) if output.status.success() => CliStatus { available: true, path: path.to_string(), error: None },
        Ok(_) => CliStatus { available: false, path: path.to_string(), error: Some(format!("{path} not found in PATH")) },
        Err(e) => CliStatus { available: false, path: path.to_string(), error: Some(format!("Failed to check {path} with {probe}: {e}")) },
    }
}
pub(crate) async fn check_cli_health_inner(settings: &AppSettings) -> Result<CliHealth, String> {
    let (mut claude, mut codex, mut gemini, mut kimi, mut opencode) = tokio::join!(
        check_single_cli(&settings.claude_path),
        check_single_cli(&settings.codex_path),
        check_single_cli(&settings.gemini_path),
        check_single_cli(&settings.kimi_path),
        check_single_cli(&settings.opencode_path),
    );
    if cfg!(target_os = "windows") && !check_binary_exists("bash").await {
        let required = Some("Git Bash is required on Windows to run agents".to_string());
        for status in [&mut claude, &mut codex, &mut gemini, &mut kimi, &mut opencode] { status.available = false; status.error = required.clone(); }
    }
    Ok(CliHealth {
        claude,
        codex,
        gemini,
        kimi,
        opencode,
    })
}
async fn update_with_npm(npm_package: &str) -> Result<String, String> {
    let output = run_npm(&["install", "-g", &format!("{npm_package}@latest")]).await?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("Update failed: {stderr}"))
}
async fn update_kimi_cli() -> Result<String, String> {
    if check_binary_exists("uv").await {
        #[cfg(target_os = "windows")]
        let output = git_bash::run_binary("uv", &["tool", "upgrade", "kimi-cli", "--no-cache"], 20)
            .await
            .ok_or_else(|| "Failed to run uv via Git Bash".to_string())?;
        #[cfg(not(target_os = "windows"))]
        let output = tokio::process::Command::new("uv")
            .args(["tool", "upgrade", "kimi-cli", "--no-cache"])
            .output()
            .await
            .map_err(|e| format!("Failed to run uv: {e}"))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Update failed: {stderr}"));
    }
    update_with_npm("kimi-cli").await
}
async fn check_binary_exists(path: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        return git_bash::command_exists(path).await;
    }
    #[cfg(not(target_os = "windows"))]
    let probe = path_probe_command();
    #[cfg(not(target_os = "windows"))]
    matches!(
        tokio::process::Command::new(probe)
            .arg(path)
            .output()
            .await,
        Ok(output) if output.status.success()
    )
}

pub(crate) async fn is_cli_available(path: &str) -> bool {
    check_binary_exists(path).await
}
async fn get_installed_version(path: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = git_bash::run_binary(path, &["--version"], 15).await?;
        if !output.status.success() {
            return None;
        }
        extract_version_from_output(&output)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let candidates = vec![path.to_string()];
        for candidate in candidates {
            let output = timeout(
                Duration::from_secs(15),
                tokio::process::Command::new(&candidate).arg("--version").output(),
            )
            .await
            .ok()?
            .ok()?;
            if output.status.success() {
                if let Some(version) = extract_version_from_output(&output) {
                    return Some(version);
                }
            }
        }
        None
    }
}
async fn get_latest_kimi_version(has_uv: bool) -> Option<String> {
    if has_uv {
        #[cfg(target_os = "windows")]
        let output = git_bash::run_binary("uvx", &["--from", "kimi-cli", "kimi", "--version"], 20).await?;
        #[cfg(not(target_os = "windows"))]
        let output = timeout(Duration::from_secs(20), tokio::process::Command::new("uvx").args(["--from", "kimi-cli", "kimi", "--version"]).output()).await.ok()?.ok()?;
        if output.status.success() {
            return extract_version_from_output(&output);
        }
    }
    get_latest_npm_version("kimi-cli").await
}
async fn build_cli_version_info(
    path: &str,
    display_name: &str,
    cli_name: &str,
    npm_package: &str,
) -> CliVersionInfo {
    let available = check_binary_exists(path).await;
    let has_uv = cli_name == "kimi" && check_binary_exists("uv").await;
    let (installed, latest) = tokio::join!(
        async {
            if available {
                get_installed_version(path).await
            } else {
                None
            }
        },
        async {
            if cli_name == "kimi" {
                get_latest_kimi_version(has_uv).await
            } else {
                get_latest_npm_version(npm_package).await
            }
        },
    );
    let up_to_date = match (&installed, &latest) {
        (Some(i), Some(l)) => i == l,
        _ => false,
    };
    let installed_and_latest_error = if cli_name == "kimi" {
        format!("Failed to read installed version and latest Kimi version for {path}")
    } else {
        format!("Failed to read installed version and latest npm version for {path}")
    };
    let latest_error = if cli_name == "kimi" {
        "Failed to fetch latest Kimi version".to_string()
    } else {
        format!("Failed to fetch latest npm version for package {npm_package}")
    };
    let error = match (available, &installed, &latest) {
        (false, _, _) => Some(format!("{path} not found in PATH")),
        (true, None, None) => Some(installed_and_latest_error),
        (true, None, Some(_)) => Some(format!("Failed to read installed version from {path} --version")),
        (true, Some(_), None) => Some(latest_error),
        (true, Some(_), Some(_)) => None,
    };
    CliVersionInfo {
        name: display_name.to_string(),
        cli_name: cli_name.to_string(),
        installed_version: installed,
        latest_version: latest,
        up_to_date,
        update_command: if has_uv { "uv tool upgrade kimi-cli --no-cache".to_string() } else { format!("npm install -g {npm_package}@latest") },
        available,
        error,
    }
}
async fn build_git_bash_version_info() -> CliVersionInfo {
    let available = check_binary_exists("bash").await;
    let (installed, latest) = tokio::join!(
        async {
            if available {
                get_installed_version("git").await
            } else {
                None
            }
        },
        get_latest_git_bash_version(),
    );
    let up_to_date = matches!((&installed, &latest), (Some(i), Some(l)) if i == l || i.starts_with(l));
    let error = match (available, &installed, &latest) {
        (false, _, _) => Some("Git Bash is required on Windows to run agents".to_string()),
        (true, None, None) => {
            Some("Failed to read installed and latest version for Git Bash".to_string())
        }
        (true, None, Some(_)) => {
            Some("Failed to read installed version from git --version".to_string())
        }
        (true, Some(_), None) => Some("Failed to fetch latest version for Git Bash".to_string()),
        (true, Some(_), Some(_)) => None,
    };
    CliVersionInfo {
        name: "Git Bash CLI".to_string(),
        cli_name: "gitBash".to_string(),
        installed_version: installed,
        latest_version: latest,
        up_to_date,
        update_command: String::new(),
        available,
        error,
    }
}
async fn get_latest_git_bash_version() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let output = git_bash::run_binary(
            "winget",
            &["show", "--id", "Git.Git", "--exact", "--accept-source-agreements", "--disable-interactivity"],
            20,
        )
        .await?;
        if !output.status.success() {
            return None;
        }
        return String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.trim().strip_prefix("Version:").map(|s| s.trim().to_string()));
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
