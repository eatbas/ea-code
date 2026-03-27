use std::sync::OnceLock;
use std::time::Duration;

/// Shared HTTP client for version-fetching operations.
/// Reuses TCP connections and TLS sessions across calls.
fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("ea-code")
            .build()
            .expect("failed to build HTTP client")
    })
}

/// Fetches JSON from `url` and extracts a version string by traversing the
/// given `json_path` keys.  Shared implementation behind the npm / PyPI helpers.
async fn fetch_version(url: &str, json_path: &[&str]) -> Option<String> {
    let resp = shared_client().get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let mut json: serde_json::Value = resp.json().await.ok()?;
    for &key in json_path {
        json = json.get(key)?.clone();
    }
    json.as_str().map(|s| s.to_string())
}

/// Fetches the latest version of an npm package from the registry.
pub(super) async fn get_latest_npm_version_http(package_name: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{package_name}/latest");
    fetch_version(&url, &["version"]).await
}

/// Fetches the latest version of a PyPI package.
pub(super) async fn get_latest_pypi_version(package_name: &str) -> Option<String> {
    let url = format!("https://pypi.org/pypi/{package_name}/json");
    fetch_version(&url, &["info", "version"]).await
}

/// Fetches the latest Git for Windows release version from GitHub API.
/// Extracts version from tag_name (e.g. "v2.47.1.windows.1" -> "2.47.1").
pub(super) async fn get_latest_git_version_http() -> Option<String> {
    let url = "https://api.github.com/repos/git-for-windows/git/releases/latest";
    let tag = fetch_version(url, &["tag_name"]).await?;
    let version = tag.trim_start_matches('v');
    version.split(".windows.").next().map(|s| s.to_string())
}
