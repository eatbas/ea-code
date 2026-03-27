//! CLI utility helpers — version extraction and npm invocation.

use crate::platform;

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

/// Runs `npm` with the given arguments, routing through the shared
/// [`platform::run_command`] utility.
pub(super) async fn run_npm(args: &[&str]) -> Result<std::process::Output, String> {
    platform::run_command("npm", args, None, 20).await
}
