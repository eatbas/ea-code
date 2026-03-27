//! Cross-platform command execution.
//!
//! Provides a single entry point for spawning CLI processes that behaves
//! correctly on both Windows (routing through Git Bash when available) and
//! Unix (spawning directly).  This replaces the duplicated platform-branching
//! logic that previously lived in `git.rs` and `commands/cli_util.rs`.

use std::process::Output;

/// Executes a CLI binary with the given arguments, optionally setting the
/// working directory, and applying a timeout.
///
/// # Platform behaviour
///
/// * **Windows** — routes through Git Bash (via [`crate::commands::git_bash`])
///   so that Unix-style CLI tools installed under Git for Windows are reachable.
/// * **Unix** — spawns the binary directly with `tokio::process::Command`.
///
/// Stdout and stderr are captured.  If the process does not complete within
/// `timeout_secs`, it is killed and an error is returned.
pub async fn run_command(
    binary: &str,
    args: &[&str],
    cwd: Option<&str>,
    timeout_secs: u64,
) -> Result<Output, String> {
    #[cfg(target_os = "windows")]
    {
        run_command_windows(binary, args, cwd, timeout_secs).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_command_unix(binary, args, cwd, timeout_secs).await
    }
}

/// Convenience wrapper that runs a command and returns its trimmed stdout on
/// success, or an error message on failure.
pub async fn run_command_stdout(
    binary: &str,
    args: &[&str],
    cwd: Option<&str>,
    timeout_secs: u64,
) -> Result<String, String> {
    let output = run_command(binary, args, cwd, timeout_secs).await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("{binary} failed: {stderr}"))
    }
}

// ---------------------------------------------------------------------------
// Windows implementation — delegates to Git Bash
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
async fn run_command_windows(
    binary: &str,
    args: &[&str],
    cwd: Option<&str>,
    timeout_secs: u64,
) -> Result<Output, String> {
    use crate::commands::cli::git_bash;

    // Build a shell script snippet that optionally `cd`s first, then execs the
    // binary.  The `exec "$0" "$@"` idiom used by `run_binary` replaces the
    // shell process with the target, keeping PID management straightforward.
    if let Some(dir) = cwd {
        // Construct a one-liner: cd '<dir>' && exec "$0" "$@"
        let script = format!("cd '{}' && exec \"$0\" \"$@\"", dir.replace('\'', "'\\''"));
        let mut bash_args: Vec<&str> = Vec::with_capacity(args.len() + 1);
        bash_args.push(binary);
        bash_args.extend_from_slice(args);
        git_bash::run_git_bash_script(&script, &bash_args, timeout_secs)
            .await
            .ok_or_else(|| format!("Failed to run {binary} via Git Bash"))
    } else {
        git_bash::run_binary(binary, args, timeout_secs)
            .await
            .ok_or_else(|| format!("Failed to run {binary} via Git Bash"))
    }
}

// ---------------------------------------------------------------------------
// Unix implementation — direct spawn
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
async fn run_command_unix(
    binary: &str,
    args: &[&str],
    cwd: Option<&str>,
    timeout_secs: u64,
) -> Result<Output, String> {
    use tokio::time::{timeout, Duration};

    let mut cmd = tokio::process::Command::new(binary);
    cmd.args(args).kill_on_drop(true);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| format!("{binary} command timed out after {timeout_secs} s"))?
        .map_err(|e| format!("Failed to run {binary}: {e}"))
}
