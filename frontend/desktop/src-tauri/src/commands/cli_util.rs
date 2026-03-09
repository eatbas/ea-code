#[cfg(target_os = "windows")]
use super::git_bash;
#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};

pub(super) fn extract_version_number(raw: &str) -> String {
    for token in raw.split_whitespace() {
        let trimmed = token.trim_start_matches('v');
        let looks_like_version =
            trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) && trimmed.contains('.');
        if looks_like_version {
            return trimmed.to_string();
        }
    }
    raw.to_string()
}

pub(super) fn extract_version_from_output(output: &std::process::Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return Some(extract_version_number(&stdout));
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    (!stderr.is_empty()).then(|| extract_version_number(&stderr))
}

pub(super) async fn run_npm(args: &[&str]) -> Result<std::process::Output, String> {
    #[cfg(target_os = "windows")]
    {
        return git_bash::run_binary("npm", args, 20)
            .await
            .ok_or_else(|| "Failed to run npm via Git Bash".to_string());
    }
    #[cfg(not(target_os = "windows"))]
    timeout(Duration::from_secs(20), tokio::process::Command::new("npm").args(args).output())
        .await
        .map_err(|_| "npm command timed out after 20 seconds".to_string())?
        .map_err(|e| format!("Failed to run npm: {e}"))
}

pub(super) async fn get_latest_npm_version(package_name: &str) -> Option<String> {
    let output = run_npm(&["view", package_name, "version"]).await.ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
