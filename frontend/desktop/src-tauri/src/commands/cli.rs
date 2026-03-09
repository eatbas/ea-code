use crate::models::*;
use tokio::time::{timeout, Duration};
fn path_probe_command() -> &'static str {
    if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    }
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
    let probe = path_probe_command();
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
    let probe = path_probe_command();
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
async fn run_npm(args: &[&str]) -> Result<std::process::Output, String> {
    if cfg!(target_os = "windows") {
        if let Ok(Ok(output)) =
            timeout(Duration::from_secs(20), tokio::process::Command::new("npm.cmd").args(args).output()).await
        {
            return Ok(output);
        }
    }
    timeout(Duration::from_secs(20), tokio::process::Command::new("npm").args(args).output()).await
        .map_err(|_| "npm command timed out after 20 seconds".to_string())?
        .map_err(|e| format!("Failed to run npm: {e}"))
}

fn extract_version_from_output(output: &std::process::Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return Some(extract_version_number(&stdout));
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    (!stderr.is_empty()).then(|| extract_version_number(&stderr))
}

async fn get_installed_version(path: &str) -> Option<String> {
    #[cfg(target_os = "windows")]
    let candidates = if std::path::Path::new(path).extension().is_none() {
        vec![
            path.to_string(),
            format!("{path}.cmd"),
            format!("{path}.exe"),
            format!("{path}.bat"),
        ]
    } else {
        vec![path.to_string()]
    };
    #[cfg(not(target_os = "windows"))]
    let candidates = vec![path.to_string()];

    for candidate in candidates {
        let output =
            timeout(Duration::from_secs(15), tokio::process::Command::new(&candidate).arg("--version").output())
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
async fn get_latest_npm_version(package_name: &str) -> Option<String> {
    let output = run_npm(&["view", package_name, "version"]).await.ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
async fn get_latest_kimi_version(has_uv: bool) -> Option<String> {
    if has_uv {
        let output = timeout(Duration::from_secs(20), tokio::process::Command::new("uvx").args(["--from", "kimi-cli", "kimi", "--version"]).output()).await.ok()?.ok()?;
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Some(extract_version_number(&raw));
        }
    }
    get_latest_npm_version("kimi-cli").await
}
fn extract_version_number(raw: &str) -> String {
    for token in raw.split_whitespace() {
        let trimmed = token.trim_start_matches('v');
        let looks_like_version =
            trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) && trimmed.contains('.');
        if looks_like_version {
            return trimmed.to_string();
        }
    }
    raw.to_string()
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
    if !cfg!(target_os = "windows") {
        return None;
    }
    let output = timeout(Duration::from_secs(20), tokio::process::Command::new("winget").args(["show", "--id", "Git.Git", "--exact", "--accept-source-agreements", "--disable-interactivity"]).output()).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("Version:").map(|s| s.trim().to_string()))
}
