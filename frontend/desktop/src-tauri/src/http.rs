//! Shared HTTP clients for the application.
//!
//! Each purpose gets a dedicated `OnceLock`-backed singleton so that TCP
//! connections and TLS sessions are reused across calls.

use std::sync::OnceLock;
use std::time::Duration;

/// General-purpose client for Symphony chat and pipeline requests.
/// No special timeout — individual callers apply per-request timeouts
/// where needed (e.g. health checks use `.timeout(Duration::from_secs(3))`).
pub fn symphony_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("failed to build Symphony HTTP client")
    })
}

/// Client for Symphony health, provider, and CLI version endpoints.
/// Uses a 120-second timeout to accommodate slow update operations.
pub fn api_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build API HTTP client")
    })
}

/// Client for external version-checking HTTP requests (npm, PyPI, Git).
/// Short timeout and custom user-agent.
pub fn version_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("maestro")
            .build()
            .expect("failed to build version HTTP client")
    })
}
