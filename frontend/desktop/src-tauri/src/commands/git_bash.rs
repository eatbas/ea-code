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
static GIT_BASH_PATH: OnceLock<Option<String>> = OnceLock::new();

#[cfg(target_os = "windows")]
pub(crate) fn find_git_bash() -> Option<&'static str> {
    GIT_BASH_PATH
        .get_or_init(find_git_bash_inner)
        .as_deref()
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

    if let Some(path) = candidates
        .into_iter()
        .find(|path| Path::new(path).exists())
    {
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
    eprintln!("[ea-code] git_bash::run: script={script:?} args={args:?} timeout={timeout_secs}s");
    let start = std::time::Instant::now();
    let result = timeout(
        Duration::from_secs(timeout_secs),
        Command::new(git_bash)
            .arg("-lc")
            .arg(script)
            .args(args)
            .stdin(Stdio::null())
            .output(),
    )
    .await;
    let elapsed = start.elapsed();
    match &result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "[ea-code] git_bash::run: completed in {elapsed:.1?} status={} stdout={:?} stderr={:?}",
                output.status, stdout.trim(), stderr.trim()
            );
        }
        Ok(Err(e)) => eprintln!("[ea-code] git_bash::run: failed in {elapsed:.1?} error={e}"),
        Err(_) => eprintln!("[ea-code] git_bash::run: TIMED OUT after {elapsed:.1?}"),
    }
    result.ok()?.ok()
}

#[cfg(target_os = "windows")]
pub(crate) async fn command_exists(binary: &str) -> bool {
    if binary.eq_ignore_ascii_case("bash") {
        return find_git_bash().is_some();
    }
    run_git_bash("command -v \"$0\" >/dev/null 2>&1", &[binary], 10)
        .await
        .is_some_and(|output| output.status.success())
}

#[cfg(target_os = "windows")]
pub(crate) async fn run_binary(binary: &str, args: &[&str], timeout_secs: u64) -> Option<Output> {
    let mut bash_args = Vec::with_capacity(args.len() + 1);
    bash_args.push(binary);
    bash_args.extend_from_slice(args);
    run_git_bash("exec \"$0\" \"$@\"", &bash_args, timeout_secs).await
}
