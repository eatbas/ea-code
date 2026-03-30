use crate::models::WorkspaceInfo;
use crate::platform;
use std::path::Path;

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
    let maestro_ignored = if is_repo {
        Some(is_maestro_ignored(path).await)
    } else {
        None
    };

    WorkspaceInfo {
        path: path.to_string(),
        is_git_repo: is_repo,
        is_dirty,
        branch,
        maestro_ignored,
    }
}

async fn is_maestro_ignored(path: &str) -> bool {
    if is_path_ignored(path, ".maestro").await.unwrap_or(false) {
        return true;
    }

    workspace_has_maestro_gitignore_entry(path).unwrap_or(false)
}

/// Runs a git command, routing through Git Bash on Windows via the shared
/// [`platform::run_command`] utility.
async fn run_git(args: &[&str]) -> Result<std::process::Output, String> {
    platform::run_command("git", args, None, 15).await
}

/// Returns whether the given relative path is ignored by git.
pub async fn is_path_ignored(path: &str, relative_path: &str) -> Result<bool, String> {
    let output = run_git(&["-C", path, "check-ignore", "-q", relative_path]).await?;
    Ok(output.status.success())
}

fn workspace_has_maestro_gitignore_entry(workspace_path: &str) -> Result<bool, String> {
    let gitignore_path = Path::new(workspace_path).join(".gitignore");
    if !gitignore_path.exists() {
        return Ok(false);
    }

    let existing = std::fs::read_to_string(&gitignore_path)
        .map_err(|error| format!("Failed to read {}: {error}", gitignore_path.display()))?;

    Ok(existing.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == ".maestro" || trimmed == ".maestro/"
    }))
}

/// Ensures the workspace root `.gitignore` contains `.maestro/`.
pub fn ensure_maestro_gitignore_entry(workspace_path: &str) -> Result<(), String> {
    let gitignore_path = Path::new(workspace_path).join(".gitignore");
    let existing = if gitignore_path.exists() {
        std::fs::read_to_string(&gitignore_path)
            .map_err(|error| format!("Failed to read {}: {error}", gitignore_path.display()))?
    } else {
        String::new()
    };

    let already_present = workspace_has_maestro_gitignore_entry(workspace_path)?;
    if already_present {
        return Ok(());
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(".maestro/\n");

    std::fs::write(&gitignore_path, updated)
        .map_err(|error| format!("Failed to write {}: {error}", gitignore_path.display()))
}
