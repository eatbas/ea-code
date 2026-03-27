//! CLI binary-existence checks, PATH resolution, and availability caching.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration as StdDuration, Instant};

#[cfg(target_os = "windows")]
use super::git_bash;
#[cfg(not(target_os = "windows"))]
use tokio::time::{timeout, Duration};

// ---------------------------------------------------------------------------
// CLI availability cache — populated by check_cli_health, reused everywhere.
// ---------------------------------------------------------------------------

static CLI_CACHE: OnceLock<Mutex<HashMap<String, (bool, Instant)>>> = OnceLock::new();
const CLI_CACHE_TTL_SECS: u64 = 7200; // 2 hours

fn cli_cache() -> &'static Mutex<HashMap<String, (bool, Instant)>> {
    CLI_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_availability(key: &str) -> Option<bool> {
    let guard = cli_cache().lock().ok()?;
    let (available, ts) = guard.get(key)?;
    (ts.elapsed() < StdDuration::from_secs(CLI_CACHE_TTL_SECS)).then_some(*available)
}

fn store_availability(key: &str, available: bool) {
    if let Ok(mut guard) = cli_cache().lock() {
        guard.insert(key.to_string(), (available, Instant::now()));
    }
}

/// Clears the cached CLI availability data so subsequent probes re-check
/// the file system / PATH.
#[tauri::command]
pub async fn invalidate_cli_cache() -> Result<(), String> {
    if let Ok(mut guard) = cli_cache().lock() {
        guard.clear();
    }
    Ok(())
}

/// Probes whether a binary is reachable on the system PATH.
///
/// On Windows this delegates to `where.exe` (or Git Bash for the special
/// `"bash"` key).  On Unix it uses `which`.  Results are cached for the
/// duration of [`CLI_CACHE_TTL_SECS`].
pub(crate) async fn check_binary_exists(path: &str) -> bool {
    if let Some(cached) = cached_availability(path) {
        return cached;
    }

    #[cfg(target_os = "windows")]
    let result = git_bash::command_exists(path).await;

    #[cfg(not(target_os = "windows"))]
    let result = {
        let probe = path_probe_command();
        let mut cmd = tokio::process::Command::new(probe);
        cmd.arg(path).kill_on_drop(true);
        matches!(
            timeout(Duration::from_secs(5), cmd.output()).await,
            Ok(Ok(output)) if output.status.success()
        )
    };

    store_availability(path, result);
    result
}

/// Convenience alias kept for call-sites that read better with this name.
pub(crate) async fn is_cli_available(path: &str) -> bool {
    check_binary_exists(path).await
}

// ---------------------------------------------------------------------------
// Platform helpers
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
fn path_probe_command() -> &'static str {
    "which"
}
