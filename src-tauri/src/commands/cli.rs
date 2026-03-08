use crate::models::*;

/// Checks whether each CLI binary is reachable.
#[tauri::command]
pub async fn check_cli_health(settings: AppSettings) -> Result<CliHealth, String> {
    check_cli_health_inner(&settings).await
}

/// Fetches version and availability information for all CLI tools.
#[tauri::command]
pub async fn get_cli_versions(settings: AppSettings) -> Result<AllCliVersions, String> {
    let (claude, codex, gemini, kimi, opencode) = tokio::join!(
        build_cli_version_info(
            &settings.claude_path,
            "Claude CLI",
            "claude",
            "@anthropic-ai/claude-code",
        ),
        build_cli_version_info(&settings.codex_path, "Codex CLI", "codex", "@openai/codex",),
        build_cli_version_info(
            &settings.gemini_path,
            "Gemini CLI",
            "gemini",
            "@google/gemini-cli",
        ),
        build_cli_version_info(&settings.kimi_path, "Kimi CLI", "kimi", "kimi-cli",),
        build_cli_version_info(
            &settings.opencode_path,
            "OpenCode CLI",
            "opencode",
            "opencode-ai",
        ),
    );

    Ok(AllCliVersions {
        claude,
        codex,
        gemini,
        kimi,
        opencode,
    })
}

/// Updates a CLI tool using its preferred package manager.
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

/// Probes a single CLI binary using `which`.
async fn check_single_cli(path: &str) -> CliStatus {
    match tokio::process::Command::new("which")
        .arg(path)
        .output()
        .await
    {
        Ok(output) if output.status.success() => CliStatus {
            available: true,
            path: path.to_string(),
            error: None,
        },
        Ok(_) => CliStatus {
            available: false,
            path: path.to_string(),
            error: Some(format!("{path} not found in PATH")),
        },
        Err(e) => CliStatus {
            available: false,
            path: path.to_string(),
            error: Some(format!("Failed to check {path}: {e}")),
        },
    }
}

/// Shared implementation for CLI health checks.
pub(crate) async fn check_cli_health_inner(settings: &AppSettings) -> Result<CliHealth, String> {
    let (claude, codex, gemini, kimi, opencode) = tokio::join!(
        check_single_cli(&settings.claude_path),
        check_single_cli(&settings.codex_path),
        check_single_cli(&settings.gemini_path),
        check_single_cli(&settings.kimi_path),
        check_single_cli(&settings.opencode_path),
    );

    Ok(CliHealth {
        claude,
        codex,
        gemini,
        kimi,
        opencode,
    })
}

/// Runs `npm install -g <package>@latest` and returns stdout on success.
async fn update_with_npm(npm_package: &str) -> Result<String, String> {
    let output = tokio::process::Command::new("npm")
        .args(["install", "-g", &format!("{npm_package}@latest")])
        .output()
        .await
        .map_err(|e| format!("Failed to run npm: {e}"))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("Update failed: {stderr}"))
}

/// Updates Kimi via `uv` when available, falling back to npm.
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
    }

    update_with_npm("kimi-cli").await
}

/// Checks whether a binary is available via `which`.
async fn check_binary_exists(path: &str) -> bool {
    matches!(
        tokio::process::Command::new("which")
            .arg(path)
            .output()
            .await,
        Ok(output) if output.status.success()
    )
}

/// Runs `<cli> --version` and extracts the version string.
async fn get_installed_version(path: &str) -> Option<String> {
    let output = tokio::process::Command::new(path)
        .arg("--version")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(extract_version_number(&raw))
}

/// Runs `npm view <package> version` to fetch the latest published version.
async fn get_latest_npm_version(package_name: &str) -> Option<String> {
    let output = tokio::process::Command::new("npm")
        .args(["view", package_name, "version"])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Extracts a semver-style version number from raw CLI output.
///
/// Handles formats like "claude v1.2.3", "1.2.3", "tool 1.2.3-beta", etc.
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

/// Builds full version information for a single CLI tool.
async fn build_cli_version_info(
    path: &str,
    display_name: &str,
    cli_name: &str,
    npm_package: &str,
) -> CliVersionInfo {
    let available = check_binary_exists(path).await;

    if !available {
        return CliVersionInfo {
            name: display_name.to_string(),
            cli_name: cli_name.to_string(),
            installed_version: None,
            latest_version: None,
            up_to_date: false,
            update_command: format!("npm install -g {npm_package}@latest"),
            available: false,
            error: Some(format!("{path} not found in PATH")),
        };
    }

    let (installed, latest) = tokio::join!(
        get_installed_version(path),
        get_latest_npm_version(npm_package),
    );

    let up_to_date = match (&installed, &latest) {
        (Some(i), Some(l)) => i == l,
        _ => false,
    };

    CliVersionInfo {
        name: display_name.to_string(),
        cli_name: cli_name.to_string(),
        installed_version: installed,
        latest_version: latest,
        up_to_date,
        update_command: format!("npm install -g {npm_package}@latest"),
        available: true,
        error: None,
    }
}
