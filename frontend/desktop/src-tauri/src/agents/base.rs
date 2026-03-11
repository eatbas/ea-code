use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
#[cfg(target_os = "windows")]
use crate::commands::git_bash::find_git_bash;
use crate::db::DbPool;
use crate::events::PipelineLogPayload;
use crate::models::PipelineStage;
#[cfg(target_os = "windows")]
const GIT_BASH_INSTALL_URL: &str = "https://git-scm.com/download/win";
/// Writes the prompt to a temp file so it can be read by bash via `$(cat ...)`,
/// avoiding Windows `CreateProcess` argument mangling for multi-line content.
#[cfg(target_os = "windows")]
fn write_prompt_temp_file(prompt: &str) -> Result<String, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Cannot determine config directory".to_string())?
        .join("ea-code")
        .join("prompts");
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

#[cfg(target_os = "windows")]
fn remove_prompt_temp_file(path: &str) {
    let _ = std::fs::remove_file(path);
}
/// Converts a Windows path like `C:\Users\...` to a Git Bash path `/c/Users/...`.
#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
fn bash_single_quote_escape(s: &str) -> String {
    s.replace('\'', "'\\''")
}

/// Builds a command that runs the given binary via Git Bash on Windows.
///
/// When `prompt_file_path` and `prompt_arg_index` are provided, the entire
/// command is constructed as a single bash script string. The prompt argument
/// is replaced with `$(cat '/path/to/file')` so that multi-line prompt content
/// is never passed through Windows `CreateProcess` argument encoding.
#[cfg(target_os = "windows")]
fn build_windows_git_bash_command(
    binary: &str,
    args: &[&str],
    prompt_file_path: Option<&str>,
    prompt_arg_index: Option<usize>,
    extra_envs: &[(&str, &str)],
) -> Result<Command, String> {
    let git_bash = find_git_bash().ok_or_else(|| {
        format!(
            "Git Bash is required on Windows to run agents. Install it: {GIT_BASH_INSTALL_URL}"
        )
    })?;
    let mut command = Command::new(git_bash);

    // Build env export prefix for extra environment variables.
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
                    // Read the prompt from a temp file, preserving newlines and
                    // special characters without any Windows argument mangling.
                    parts.push(format!(
                        "\"$(cat '{}')\"",
                        bash_single_quote_escape(&bash_path)
                    ));
                } else {
                    parts.push(format!("'{}'", bash_single_quote_escape(arg)));
                }
            }
            command.arg("-lc").arg(format!("{env_prefix}{}", parts.join(" ")));
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
                command.arg("-lc").arg(format!("{env_prefix}{}", parts.join(" ")));
            }
        }
    }

    Ok(command)
}

/// Input passed to each agent invocation.
#[derive(Clone, Debug)]
pub struct AgentInput {
    pub prompt: String,
    pub context: Option<String>,
    pub workspace_path: String,
}

/// Output captured from agent execution.
#[derive(Clone, Debug)]
pub struct AgentOutput {
    pub raw_text: String,
}

/// Assembles a full prompt by concatenating the base prompt with optional
/// context sections.
pub fn build_full_prompt(input: &AgentInput) -> String {
    let mut parts = vec![input.prompt.clone()];
    if let Some(ref ctx) = input.context {
        parts.push(format!("\n\n--- Context ---\n{ctx}"));
    }
    parts.join("")
}

/// Spawns a CLI process, streams stdout/stderr line by line, and emits
/// `pipeline:log` events for each line. Returns the captured output and
/// exit code.
///
/// `prompt_arg_index` indicates which element of `args` contains the prompt
/// text. On Windows, that argument is written to a temp file and read back
/// by bash via `$(cat ...)` to avoid `CreateProcess` mangling multi-line
/// strings. On Unix this parameter is ignored.
///
/// `stdin_text`, when provided, is written to the child process's stdin and
/// then stdin is closed. This is the preferred way to pass prompts to CLIs
/// that support piped input (Codex, Gemini, Kimi, OpenCode). When using
/// stdin, set `prompt_arg_index` to `None` since the prompt is not in args.
///
/// `extra_envs` are additional environment variables to set on the child
/// process (e.g. `PYTHONIOENCODING=utf-8` for Kimi).
pub async fn run_cli_agent(
    binary: &str,
    args: &[&str],
    _prompt_arg_index: Option<usize>,
    workspace_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    _db: &DbPool,
    stdin_text: Option<&str>,
    extra_envs: &[(&str, &str)],
) -> Result<AgentOutput, String> {
    #[cfg(target_os = "windows")]
    let prompt_file: Option<String> = match _prompt_arg_index {
        Some(idx) => Some(write_prompt_temp_file(args[idx])?),
        None => None,
    };

    #[cfg(target_os = "windows")]
    let mut command =
        build_windows_git_bash_command(binary, args, prompt_file.as_deref(), _prompt_arg_index, extra_envs)?;

    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut command = Command::new(binary);
        command.args(args);
        for &(key, value) in extra_envs {
            command.env(key, value);
        }
        command
    };

    // Pipe stdin when we need to write prompt text to the child.
    if stdin_text.is_some() {
        command.stdin(std::process::Stdio::piped());
    }

    let mut child = command
        .current_dir(workspace_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {binary}: {e}"))?;

    // Write prompt to stdin concurrently with reading stdout/stderr
    // to avoid deadlocks when the child's output buffer fills up.
    let stdin_handle = if let Some(text) = stdin_text {
        let mut child_stdin = child.stdin.take()
            .ok_or_else(|| format!("Failed to capture stdin for {binary}"))?;
        let text = text.to_string();
        Some(tokio::spawn(async move {
            let _ = child_stdin.write_all(text.as_bytes()).await;
            let _ = child_stdin.shutdown().await;
        }))
    } else {
        None
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Failed to capture stdout from {binary}"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("Failed to capture stderr from {binary}"))?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let app_out = app.clone();
    let run_id_out = run_id.to_string();
    let stage_out = stage.clone();

    let app_err = app.clone();
    let run_id_err = run_id.to_string();
    let stage_err = stage.clone();

    // Read stdout and stderr concurrently via separate tasks
    let stdout_handle = tokio::spawn(async move {
        let mut lines = Vec::new();
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            let _ = app_out.emit(
                "pipeline:log",
                PipelineLogPayload {
                    run_id: run_id_out.clone(),
                    stage: stage_out.clone(),
                    line: line.clone(),
                    stream: "stdout".to_string(),
                },
            );
            lines.push(line);
        }
        lines
    });

    let stderr_handle = tokio::spawn(async move {
        let mut lines = Vec::new();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            let _ = app_err.emit(
                "pipeline:log",
                PipelineLogPayload {
                    run_id: run_id_err.clone(),
                    stage: stage_err.clone(),
                    line: line.clone(),
                    stream: "stderr".to_string(),
                },
            );
            lines.push(line);
        }
        lines
    });

    // Wait for stdin writer to finish (if active).
    if let Some(handle) = stdin_handle {
        let _ = handle.await;
    }

    let stdout_lines = stdout_handle
        .await
        .map_err(|e| format!("stdout reader task failed: {e}"))?;
    let stderr_lines = stderr_handle
        .await
        .map_err(|e| format!("stderr reader task failed: {e}"))?;

    let stdout_output = stdout_lines.join("\n");
    let stderr_output = stderr_lines.join("\n");

    // Wait for process to exit before returning output.
    let status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for {binary}: {e}"))?;

    // Clean up the prompt temp file after the process exits.
    #[cfg(target_os = "windows")]
    if let Some(ref pf) = prompt_file {
        remove_prompt_temp_file(pf);
    }

    if !status.success() {
        let output = if stdout_output.trim().is_empty() {
            stderr_output.trim()
        } else {
            stdout_output.trim()
        };
        let details = if output.is_empty() {
            "No output captured".to_string()
        } else {
            output.to_string()
        };
        return Err(format!(
            "{binary} exited with status {}: {details}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "terminated by signal".to_string())
        ));
    }

    Ok(AgentOutput {
        // Keep stage output clean: terminal stderr remains in live logs only.
        raw_text: stdout_output,
    })
}
