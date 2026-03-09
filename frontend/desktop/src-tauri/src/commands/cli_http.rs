use std::time::Duration;

/// Fetches the latest version of an npm package from the registry.
/// Queries: GET https://registry.npmjs.org/<package>/latest
pub(super) async fn get_latest_npm_version_http(package_name: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{package_name}/latest");
    eprintln!("[ea-code] http: fetching {url}");
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        eprintln!("[ea-code] http: {url} returned status {}", resp.status());
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let version = json["version"].as_str().map(|s| s.to_string());
    eprintln!("[ea-code] http: {package_name} => {version:?} ({:.1?})", start.elapsed());
    version
}

/// Fetches the latest version of a PyPI package.
/// Queries: GET https://pypi.org/pypi/<package>/json
pub(super) async fn get_latest_pypi_version(package_name: &str) -> Option<String> {
    let url = format!("https://pypi.org/pypi/{package_name}/json");
    eprintln!("[ea-code] http: fetching {url}");
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        eprintln!("[ea-code] http: {url} returned status {}", resp.status());
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let version = json["info"]["version"].as_str().map(|s| s.to_string());
    eprintln!("[ea-code] http: {package_name} (pypi) => {version:?} ({:.1?})", start.elapsed());
    version
}

/// Fetches the latest Git for Windows release version from GitHub API.
/// Queries: GET https://api.github.com/repos/git-for-windows/git/releases/latest
/// Extracts version from tag_name (e.g. "v2.47.1.windows.1" -> "2.47.1").
pub(super) async fn get_latest_git_version_http() -> Option<String> {
    let url = "https://api.github.com/repos/git-for-windows/git/releases/latest";
    eprintln!("[ea-code] http: fetching {url}");
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("ea-code")
        .build()
        .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        eprintln!("[ea-code] http: {url} returned status {}", resp.status());
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let tag = json["tag_name"].as_str()?;
    let version = tag.trim_start_matches('v');
    let result = version.split(".windows.").next().map(|s| s.to_string());
    eprintln!("[ea-code] http: git-for-windows => {result:?} ({:.1?})", start.elapsed());
    result
}
