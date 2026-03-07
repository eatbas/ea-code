use std::process::Command;

use crate::models::WorkspaceInfo;

/// Checks whether the given path resides inside a git repository.
pub fn is_git_repo(path: &str) -> bool {
    Command::new("git")
        .args(["-C", path, "rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// Returns the porcelain status output for the workspace.
pub fn git_status(path: &str) -> String {
    Command::new("git")
        .args(["-C", path, "status", "--porcelain"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
        .unwrap_or_default()
}

/// Returns the diff of all changes (staged and unstaged) in the workspace.
pub fn git_diff(path: &str) -> String {
    Command::new("git")
        .args(["-C", path, "diff"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
        .unwrap_or_default()
}

/// Returns the current branch name, if available.
pub fn git_branch(path: &str) -> Option<String> {
    Command::new("git")
        .args(["-C", path, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Gathers workspace information including git status.
pub fn workspace_info(path: &str) -> WorkspaceInfo {
    let is_repo = is_git_repo(path);
    let is_dirty = if is_repo {
        !git_status(path).trim().is_empty()
    } else {
        false
    };
    let branch = if is_repo { git_branch(path) } else { None };

    WorkspaceInfo {
        path: path.to_string(),
        is_git_repo: is_repo,
        is_dirty,
        branch,
    }
}
