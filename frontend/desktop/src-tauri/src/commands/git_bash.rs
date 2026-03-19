#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::process::{Output, Stdio};
#[cfg(target_os = "windows")]
use std::sync::OnceLock;
#[cfg(target_os = "windows")]
use tokio::process::Command;
#[cfg(target_os = "windows")]
use tokio::time::{timeout, Duration};
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
fn terminate_process_tree(pid: u32) {
    let pid_arg = pid.to_string();
    let _ = std::process::Command::new("taskkill")
        .args(["/T", "/F", "/PID", pid_arg.as_str()])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(target_os = "windows")]
static GIT_BASH_PATH: OnceLock<Option<String>> = OnceLock::new();

#[cfg(target_os = "windows")]
pub(crate) fn find_git_bash() -> Option<&'static str> {
    GIT_BASH_PATH.get_or_init(find_git_bash_inner).as_deref()
}

#[cfg(target_os = "windows")]
fn find_git_bash_inner() -> Option<String> {
    let mut candidates = Vec::new();
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        candidates.push(format!("{program_files}\\Git\\bin\\bash.exe"));
    }
    if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
        candidates.push(format!("{program_files_x86}\\Git\\bin\\bash.exe"));
    }
    if let Ok(local_app_data) = std::env::var("LocalAppData") {
        candidates.push(format!("{local_app_data}\\Programs\\Git\\bin\\bash.exe"));
    }

    if let Some(path) = candidates.into_iter().find(|path| Path::new(path).exists()) {
        return Some(path);
    }

    let found: Vec<String> = std::env::var_os("PATH")
        .map(|path_var| {
            std::env::split_paths(&path_var)
                .map(|dir| dir.join("bash.exe"))
                .filter(|candidate| candidate.exists())
                .map(|candidate| candidate.to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    if let Some(preferred) = found
        .iter()
        .find(|line| line.to_ascii_lowercase().contains("\\git\\"))
        .cloned()
        .or_else(|| found.into_iter().next())
    {
        return Some(preferred);
    }

    let output = std::process::Command::new("cmd")
        .args(["/C", "where bash.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let located: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && Path::new(line).exists())
        .map(str::to_string)
        .collect();
    located
        .iter()
        .find(|line| line.to_ascii_lowercase().contains("\\git\\"))
        .cloned()
        .or_else(|| located.into_iter().next())
}

#[cfg(target_os = "windows")]
async fn run_git_bash(script: &str, args: &[&str], timeout_secs: u64) -> Option<Output> {
    let git_bash = find_git_bash()?;
    let mut command = Command::new(git_bash);
    command
        .arg("-c")
        .arg(script)
        .args(args)
        .stdin(Stdio::null())
        // Tauri GUI processes on Windows may not have a valid inherited stdout/stderr.
        // Always pipe child output so CLIs (for example Kimi via Python/Colorama) can write safely.
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true);

    let child = command.spawn().ok()?;
    let child_pid = child.id();

    match timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await {
        Ok(output) => output.ok(),
        Err(_) => {
            if let Some(pid) = child_pid {
                terminate_process_tree(pid);
            }
            None
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) async fn command_exists(binary: &str) -> bool {
    if binary.eq_ignore_ascii_case("bash") {
        return find_git_bash().is_some();
    }
    // Full path: check file existence directly (no spawn).
    if binary.contains('\\') || binary.contains('/') {
        return Path::new(binary).exists();
    }
    // Use native where.exe for PATH lookup — avoids spawning Git Bash entirely.
    timeout(
        Duration::from_secs(5),
        Command::new("where.exe")
            .arg(binary)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .kill_on_drop(true)
            .status(),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
    .is_some_and(|s| s.success())
}

#[cfg(target_os = "windows")]
pub(crate) async fn run_binary(binary: &str, args: &[&str], timeout_secs: u64) -> Option<Output> {
    let mut bash_args = Vec::with_capacity(args.len() + 1);
    bash_args.push(binary);
    bash_args.extend_from_slice(args);
    run_git_bash("exec \"$0\" \"$@\"", &bash_args, timeout_secs).await
}
