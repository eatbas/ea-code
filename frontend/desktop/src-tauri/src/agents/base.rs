use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db::{self, DbPool};
use crate::events::PipelineLogPayload;
use crate::models::PipelineStage;

#[cfg(target_os = "windows")]
const GIT_BASH_INSTALL_URL: &str = "https://git-scm.com/download/win";

#[cfg(target_os = "windows")]
fn find_git_bash() -> Option<String> {
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

    for path in candidates {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    let where_output = std::process::Command::new("where")
        .arg("bash")
        .output()
        .ok()?;
    if !where_output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&where_output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| line.to_ascii_lowercase().contains("\\git\\") && !line.is_empty())
        .map(str::to_string)
}

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
) -> Result<Command, String> {
    let git_bash = find_git_bash().ok_or_else(|| {
        format!(
            "Git Bash is required on Windows to run agents. Install it: {GIT_BASH_INSTALL_URL}"
        )
    })?;
    let mut command = Command::new(git_bash);

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
            command.arg("-lc").arg(parts.join(" "));
        }
        _ => {
            command
                .arg("-lc")
                .arg("exec \"$0\" \"$@\"")
                .arg(binary)
                .args(args);
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
pub async fn run_cli_agent(
    binary: &str,
    args: &[&str],
    _prompt_arg_index: Option<usize>,
    workspace_path: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    db: &DbPool,
) -> Result<AgentOutput, String> {
    #[cfg(target_os = "windows")]
    let prompt_file: Option<String> = match _prompt_arg_index {
        Some(idx) => Some(write_prompt_temp_file(args[idx])?),
        None => None,
    };

    #[cfg(target_os = "windows")]
    let mut command =
        build_windows_git_bash_command(binary, args, prompt_file.as_deref(), _prompt_arg_index)?;

    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut command = Command::new(binary);
        command.args(args);
        command
    };

    let mut child = command
        .current_dir(workspace_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {binary}: {e}"))?;

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

    let stage_str = serde_json::to_value(&stage)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("{stage:?}"));

    let db_out = db.clone();
    let stage_str_out = stage_str.clone();

    let db_err = db.clone();
    let stage_str_err = stage_str;

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
            // Fire-and-forget log persistence
            let _ = db::logs::insert(&db_out, &run_id_out, &stage_str_out, &line, "stdout");
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
            let _ = db::logs::insert(&db_err, &run_id_err, &stage_str_err, &line, "stderr");
            lines.push(line);
        }
        lines
    });

    let stdout_lines = stdout_handle
        .await
        .map_err(|e| format!("stdout reader task failed: {e}"))?;
    let stderr_lines = stderr_handle
        .await
        .map_err(|e| format!("stderr reader task failed: {e}"))?;

    let mut all_output = stdout_lines.join("\n");
    if !stderr_lines.is_empty() {
        if !all_output.is_empty() {
            all_output.push('\n');
        }
        all_output.push_str(&stderr_lines.join("\n"));
    }

    // Wait for process to exit before returning output.
    let _status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for {binary}: {e}"))?;

    // Clean up the prompt temp file after the process exits.
    #[cfg(target_os = "windows")]
    if let Some(ref pf) = prompt_file {
        remove_prompt_temp_file(pf);
    }

    Ok(AgentOutput {
        raw_text: all_output,
    })
}
