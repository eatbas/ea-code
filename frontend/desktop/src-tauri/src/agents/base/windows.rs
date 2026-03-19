use tokio::process::Command;

use crate::commands::git_bash::find_git_bash;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const GIT_BASH_INSTALL_URL: &str = "https://git-scm.com/download/win";

/// Writes the prompt to a temp file so it can be read by bash via `$(cat ...)`,
/// avoiding Windows `CreateProcess` argument mangling for multi-line content.
pub(super) fn write_prompt_temp_file(prompt: &str) -> Result<String, String> {
    let config_dir = crate::storage::config_dir()?.join("prompts");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create prompt temp directory: {e}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file_path = config_dir.join(format!("prompt-{stamp}.txt"));
    std::fs::write(&file_path, prompt)
        .map_err(|e| format!("Failed to write prompt temp file: {e}"))?;
    Ok(file_path.to_string_lossy().to_string())
}

pub(super) fn remove_prompt_temp_file(path: &str) {
    let _ = std::fs::remove_file(path);
}

/// Converts a Windows path like `C:\Users\...` to a Git Bash path `/c/Users/...`.
fn windows_path_to_bash_path(windows_path: &str) -> String {
    let path = windows_path.replace('\\', "/");
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        let drive = path.as_bytes()[0].to_ascii_lowercase() as char;
        format!("/{drive}{}", &path[2..])
    } else {
        path
    }
}

/// Escapes a string for use inside bash single quotes.
fn bash_single_quote_escape(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// Builds a command that runs the given binary via Git Bash on Windows.
///
/// When `prompt_file_path` and `prompt_arg_index` are provided, the entire
/// command is constructed as a single bash script string. The prompt argument
/// is replaced with `$(cat '/path/to/file')` so that multi-line prompt content
/// is never passed through Windows `CreateProcess` argument encoding.
pub(super) fn build_windows_git_bash_command(
    binary: &str,
    args: &[&str],
    prompt_file_path: Option<&str>,
    prompt_arg_index: Option<usize>,
    extra_envs: &[(&str, &str)],
) -> Result<Command, String> {
    let git_bash = find_git_bash().ok_or_else(|| {
        format!("Git Bash is required on Windows to run agents. Install it: {GIT_BASH_INSTALL_URL}")
    })?;

    let mut command = Command::new(git_bash);
    command.creation_flags(CREATE_NO_WINDOW);

    let env_prefix = if extra_envs.is_empty() {
        String::new()
    } else {
        let exports: Vec<String> = extra_envs
            .iter()
            .map(|(k, v)| format!("export {}='{}'", k, bash_single_quote_escape(v)))
            .collect();
        format!("{}; ", exports.join("; "))
    };

    match (prompt_file_path, prompt_arg_index) {
        (Some(pf), Some(idx)) => {
            let bash_path = windows_path_to_bash_path(pf);
            let mut parts = vec![format!("exec '{}'", bash_single_quote_escape(binary))];
            for (i, arg) in args.iter().enumerate() {
                if i == idx {
                    parts.push(format!(
                        "\"$(cat '{}')\"",
                        bash_single_quote_escape(&bash_path)
                    ));
                } else {
                    parts.push(format!("'{}'", bash_single_quote_escape(arg)));
                }
            }
            command
                .arg("-lc")
                .arg(format!("{env_prefix}{}", parts.join(" ")));
        }
        _ => {
            if extra_envs.is_empty() {
                command
                    .arg("-lc")
                    .arg("exec \"$0\" \"$@\"")
                    .arg(binary)
                    .args(args);
            } else {
                let mut parts = vec![format!("exec '{}'", bash_single_quote_escape(binary))];
                for arg in args {
                    parts.push(format!("'{}'", bash_single_quote_escape(arg)));
                }
                command
                    .arg("-lc")
                    .arg(format!("{env_prefix}{}", parts.join(" ")));
            }
        }
    }

    Ok(command)
}
