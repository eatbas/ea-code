use crate::models::WorkspaceInfo;
use crate::platform;

/// Checks whether the given path resides inside a git repository.
pub async fn is_git_repo(path: &str) -> bool {
    run_git(&["-C", path, "rev-parse", "--is-inside-work-tree"])
        .await
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Returns the porcelain status output for the workspace.
pub async fn git_status(path: &str) -> String {
    run_git(&["-C", path, "status", "--porcelain"])
        .await
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
        .unwrap_or_default()
}

/// Returns the current branch name, if available.
pub async fn git_branch(path: &str) -> Option<String> {
    run_git(&["-C", path, "rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Gathers workspace information including git status.
pub async fn workspace_info(path: &str) -> WorkspaceInfo {
    let is_repo = is_git_repo(path).await;
    let is_dirty = if is_repo {
        !git_status(path).await.trim().is_empty()
    } else {
        false
    };
    let branch = if is_repo {
        git_branch(path).await
    } else {
        None
    };

    WorkspaceInfo {
        path: path.to_string(),
        is_git_repo: is_repo,
        is_dirty,
        branch,
    }
}

/// Runs a git command, routing through Git Bash on Windows via the shared
/// [`platform::run_command`] utility.
async fn run_git(args: &[&str]) -> Result<std::process::Output, String> {
    platform::run_command("git", args, None, 15).await
}
