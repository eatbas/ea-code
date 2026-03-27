use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Result of Python detection: the command and optional version flag for `py` launcher.
#[derive(Debug, Clone)]
pub struct PythonInterpreter {
    /// The executable name or path (e.g. "python", "py", "python3.13").
    pub executable: String,
    /// Optional version flag for the Windows `py` launcher (e.g. "-3.13").
    pub launcher_version: Option<String>,
}

impl PythonInterpreter {
    /// Build the command to create a venv.
    pub fn venv_command(&self, venv_dir: &Path) -> Command {
        let mut cmd = Command::new(&self.executable);
        if let Some(ref ver) = self.launcher_version {
            cmd.arg(ver);
        }
        cmd.args(["-m", "venv"]);
        cmd.arg(venv_dir);
        cmd
    }

    /// Build a command to run an arbitrary Python module with the *system* interpreter.
    pub fn run_module(&self, module: &str) -> Command {
        let mut cmd = Command::new(&self.executable);
        if let Some(ref ver) = self.launcher_version {
            cmd.arg(ver);
        }
        cmd.args(["-m", module]);
        cmd
    }
}

/// Locate a Python >= 3.12 interpreter on the system.
///
/// Search order:
/// 1. Windows `py` launcher with explicit version flags (3.14, 3.13, 3.12).
/// 2. macOS well-known Homebrew paths (Apple Silicon `/opt/homebrew`, Intel `/usr/local`).
/// 3. Unix-style versioned binaries on PATH: python3.14, python3.13, python3.12.
/// 4. Generic python3 / python on PATH — only accepted if >= 3.12.
pub async fn find_python() -> Result<PythonInterpreter, String> {
    // 1. Windows py launcher
    #[cfg(target_os = "windows")]
    {
        if binary_exists("py").await {
            for ver in ["3.14", "3.13", "3.12"] {
                let flag = format!("-{ver}");
                let ok = Command::new("py")
                    .args([&flag, "--version"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .await
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok {
                    return Ok(PythonInterpreter {
                        executable: "py".into(),
                        launcher_version: Some(flag),
                    });
                }
            }
        }
    }

    // 2. macOS Homebrew well-known paths (faster than PATH search)
    #[cfg(target_os = "macos")]
    {
        // Apple Silicon: /opt/homebrew/bin/python3*
        // Intel Mac:     /usr/local/bin/python3*
        let brew_prefixes = ["/opt/homebrew/bin", "/usr/local/bin"];
        for prefix in &brew_prefixes {
            for ver in ["python3.14", "python3.13", "python3.12"] {
                let path = format!("{prefix}/{ver}");
                if check_python_version(&path).await {
                    return Ok(PythonInterpreter {
                        executable: path,
                        launcher_version: None,
                    });
                }
            }
            // Also try the unversioned python3 at this prefix
            let path = format!("{prefix}/python3");
            if check_python_version(&path).await {
                return Ok(PythonInterpreter {
                    executable: path,
                    launcher_version: None,
                });
            }
        }
    }

    // 3. Versioned binaries on PATH
    for candidate in ["python3.14", "python3.13", "python3.12"] {
        if binary_exists(candidate).await {
            return Ok(PythonInterpreter {
                executable: candidate.into(),
                launcher_version: None,
            });
        }
    }

    // 4. Generic python3 / python on PATH — version check
    for candidate in ["python3", "python"] {
        if binary_exists(candidate).await && check_python_version(candidate).await {
            return Ok(PythonInterpreter {
                executable: candidate.into(),
                launcher_version: None,
            });
        }
    }

    Err(
        "Python 3.12 or newer is required but was not found. \
         Install from https://www.python.org/downloads/"
            .into(),
    )
}

/// Resolve the venv Python executable path (platform-aware).
pub fn venv_python(venv_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        let scripts = venv_dir.join("Scripts").join("python.exe");
        if scripts.exists() {
            return scripts;
        }
    }
    venv_dir.join("bin").join("python")
}

/// Check whether a venv exists and has a working Python >= 3.12.
pub async fn venv_is_valid(venv_dir: &Path) -> bool {
    let py = venv_python(venv_dir);
    if !py.exists() {
        return false;
    }
    check_python_version(py.to_string_lossy().as_ref()).await
}

// ── helpers ──────────────────────────────────────────────────────────

async fn binary_exists(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where.exe")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

async fn check_python_version(executable: &str) -> bool {
    Command::new(executable)
        .args(["-c", "import sys; exit(0 if sys.version_info >= (3,12) else 1)"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}
