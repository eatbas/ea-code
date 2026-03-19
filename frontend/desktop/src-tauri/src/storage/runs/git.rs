use std::process::Command;

use crate::models::{GitBaseline, RunFileStatus, RunSummary};

/// Captures the current git baseline (HEAD SHA and dirty state).
/// Executes git commands in the specified workspace directory.
/// Returns None if git is not installed or not a git repository.
pub fn capture_git_baseline(workspace_path: &str) -> Result<Option<GitBaseline>, String> {
    // Try to get HEAD SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace_path)
        .output();

    let commit_sha = match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(_) => return Ok(None), // Git command ran but failed - not a git repo
        Err(_) => return Ok(None), // Git not installed or not in PATH
    };

    // Check if working tree has unstaged changes
    let output = Command::new("git")
        .args(["diff", "--quiet"])
        .current_dir(workspace_path)
        .output();

    let had_unstaged_changes = match output {
        Ok(output) => !output.status.success(), // Exit code 1 means there are changes
        _ => false,
    };

    // Also check for staged-but-uncommitted changes
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(workspace_path)
        .output();

    let had_staged_changes = match output {
        Ok(output) => !output.status.success(), // Exit code 1 means there are staged changes
        _ => false,
    };

    Ok(Some(GitBaseline {
        commit_sha,
        had_unstaged_changes: had_unstaged_changes || had_staged_changes,
    }))
}

/// Computes files changed for a run using git diff from the baseline.
/// Handles both clean and dirty working tree scenarios, with fallback to
/// recent commits when baseline comparison is unreliable.
pub fn compute_files_changed(summary: &RunSummary) -> Result<Vec<String>, String> {
    let Some(ref baseline) = summary.git_baseline else {
        return Ok(Vec::new());
    };

    let workspace = summary
        .workspace_path
        .as_ref()
        .ok_or_else(|| "No workspace path".to_string())?;

    // Try baseline comparison first
    let output = Command::new("git")
        .current_dir(workspace)
        .args([
            "diff",
            "--name-only",
            &format!("{}..HEAD", baseline.commit_sha),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let files = parse_git_diff_output(&out.stdout);

            // If tree was dirty at start, also include working tree changes
            if baseline.had_unstaged_changes {
                let wt_files = get_working_tree_changes(workspace)?;
                // Merge and deduplicate
                return Ok(merge_file_lists(files, wt_files));
            }

            Ok(files)
        }
        _ => {
            // Fallback: list files from recent commits
            get_files_from_recent_commits(workspace, 5)
        }
    }
}

/// Internal version that works with explicit baseline and workspace.
/// This maintains backward compatibility with existing callers.
pub fn compute_files_changed_internal(
    baseline: &GitBaseline,
    workspace_path: &str,
) -> Result<Vec<String>, String> {
    // Create a minimal RunSummary to delegate to the main implementation
    let summary = RunSummary {
        schema_version: 1,
        id: String::new(),
        session_id: String::new(),
        prompt: String::new(),
        enhanced_prompt: None,
        status: RunFileStatus::Completed,
        final_verdict: None,
        current_stage: None,
        current_iteration: None,
        total_iterations: 0,
        max_iterations: 0,
        executive_summary: None,
        files_changed: Vec::new(),
        error: None,
        git_baseline: Some(baseline.clone()),
        workspace_path: Some(workspace_path.to_string()),
        next_sequence: 1,
        started_at: String::new(),
        completed_at: None,
    };

    compute_files_changed(&summary)
}

/// Gets the list of files changed in the working tree (uncommitted changes).
fn get_working_tree_changes(workspace: &str) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(["diff", "--name-only", "HEAD"])
        .output()
        .map_err(|e| format!("Git diff failed: {e}"))?;

    if !output.status.success() {
        return Err("Git diff HEAD failed".to_string());
    }

    Ok(parse_git_diff_output(&output.stdout))
}

/// Gets the list of files changed in the most recent commits.
fn get_files_from_recent_commits(workspace: &str, count: usize) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(workspace)
        .args(["diff", "--name-only", &format!("HEAD~{}..HEAD", count)])
        .output()
        .map_err(|e| format!("Git diff failed: {e}"))?;

    if !output.status.success() {
        // If HEAD~N doesn't exist (e.g., shallow clone with few commits),
        // fall back to just listing all files in the last commit
        let fallback = Command::new("git")
            .current_dir(workspace)
            .args(["diff", "--name-only", "HEAD^..HEAD"])
            .output();

        match fallback {
            Ok(out) if out.status.success() => Ok(parse_git_diff_output(&out.stdout)),
            _ => {
                // Last resort: show all tracked files
                let ls_output = Command::new("git")
                    .current_dir(workspace)
                    .args(["ls-files"])
                    .output()
                    .map_err(|e| format!("Git ls-files failed: {e}"))?;

                Ok(parse_git_diff_output(&ls_output.stdout))
            }
        }
    } else {
        Ok(parse_git_diff_output(&output.stdout))
    }
}

/// Parses git diff output into a list of file paths.
fn parse_git_diff_output(stdout: &[u8]) -> Vec<String> {
    let stdout = String::from_utf8_lossy(stdout);
    stdout
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Merges two file lists and removes duplicates while preserving order.
fn merge_file_lists(list1: Vec<String>, list2: Vec<String>) -> Vec<String> {
    let mut result = list1;
    for file in list2 {
        if !result.contains(&file) {
            result.push(file);
        }
    }
    result
}
