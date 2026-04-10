use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// ── Python path cache ──────────────────────────────────────────────

const CACHE_FILE: &str = ".python_cache.json";
const CACHE_MAX_AGE_SECS: u64 = 86_400; // 24 hours

#[derive(Serialize, Deserialize)]
struct PythonCache {
    executable: String,
    launcher_version: Option<String>,
    /// Unix epoch seconds when the cache was written.
    cached_at: u64,
}

fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".maestro").join(CACHE_FILE))
}

fn read_python_cache() -> Option<PythonCache> {
    let path = cache_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Persist the discovered Python interpreter for faster subsequent launches.
pub fn write_python_cache(interp: &PythonInterpreter) {
    let Some(path) = cache_path() else { return };
    let cache = PythonCache {
        executable: interp.executable.clone(),
        launcher_version: interp.launcher_version.clone(),
        cached_at: epoch_secs(),
    };
    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = std::fs::write(&path, json);
    }
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Venv validation marker ─────────────────────────────────────────

const VENV_MARKER: &str = ".symphony_validated";

/// Result of Python detection: the command and optional version flag for `py` launcher.
#[derive(Debug, Clone)]
pub struct PythonInterpreter {
    /// The executable name or absolute path (e.g. "py", "C:\\Python312\\python.exe").
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
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }

    /// Build a command to run an arbitrary Python module with the *system* interpreter.
    pub fn run_module(&self, module: &str) -> Command {
        let mut cmd = Command::new(&self.executable);
        if let Some(ref ver) = self.launcher_version {
            cmd.arg(ver);
        }
        cmd.args(["-m", module]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
}

/// Locate a Python >= 3.12 interpreter on the system.
///
/// Search order:
/// 1. Windows `py` launcher with explicit version flags (3.14, 3.13, 3.12).
/// 2. Windows well-known install locations and `where.exe` resolved absolute paths.
/// 3. macOS well-known Homebrew paths (Apple Silicon `/opt/homebrew`, Intel `/usr/local`).
/// 4. Unix-style versioned binaries on PATH: python3.14, python3.13, python3.12.
/// 5. Generic python3 / python on PATH — only accepted if >= 3.12.
pub async fn find_python() -> Result<PythonInterpreter, String> {
    // Fast path: try the cached interpreter from a previous launch.
    // This turns 9 sequential subprocess spawns into 1 validation call.
    if let Some(cache) = read_python_cache() {
        if epoch_secs().saturating_sub(cache.cached_at) < CACHE_MAX_AGE_SECS
            && check_python_version(&cache.executable).await
        {
            return Ok(PythonInterpreter {
                executable: cache.executable,
                launcher_version: cache.launcher_version,
            });
        }
    }

    // 1. Windows py launcher
    #[cfg(target_os = "windows")]
    {
        if binary_exists("py").await {
            for ver in ["3.14", "3.13", "3.12"] {
                let flag = format!("-{ver}");
                let ok = Command::new("py")
                    .args([&flag, "--version"])
                    .creation_flags(CREATE_NO_WINDOW)
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

    // 2. Windows: check well-known install paths and resolve via where.exe
    #[cfg(target_os = "windows")]
    {
        // Common installer locations
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let program_files = std::env::var("ProgramFiles").unwrap_or_default();

        for ver in ["314", "313", "312"] {
            let candidates = [
                format!("{local_app_data}\\Programs\\Python\\Python{ver}\\python.exe"),
                format!("{program_files}\\Python{ver}\\python.exe"),
            ];
            for path in &candidates {
                if check_python_version(path).await {
                    return Ok(PythonInterpreter {
                        executable: path.clone(),
                        launcher_version: None,
                    });
                }
            }
        }

        // Ask each candidate for its real sys.executable path, which
        // resolves the actual binary behind WindowsApps aliases.
        for candidate in ["python3", "python"] {
            if let Some(real_path) = resolve_real_python_path(candidate).await {
                if check_python_version(&real_path).await {
                    return Ok(PythonInterpreter {
                        executable: real_path,
                        launcher_version: None,
                    });
                }
            }
        }
    }

    // 3. macOS Homebrew well-known paths (faster than PATH search)
    #[cfg(target_os = "macos")]
    {
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
            let path = format!("{prefix}/python3");
            if check_python_version(&path).await {
                return Ok(PythonInterpreter {
                    executable: path,
                    launcher_version: None,
                });
            }
        }
    }

    // 4. Versioned binaries on PATH
    for candidate in ["python3.14", "python3.13", "python3.12"] {
        if binary_exists(candidate).await {
            return Ok(PythonInterpreter {
                executable: candidate.into(),
                launcher_version: None,
            });
        }
    }

    // 5. Generic python3 / python on PATH — version check
    for candidate in ["python3", "python"] {
        if binary_exists(candidate).await && check_python_version(candidate).await {
            return Ok(PythonInterpreter {
                executable: candidate.into(),
                launcher_version: None,
            });
        }
    }

    Err("Python 3.12 or newer is required but was not found. \
         Install from https://www.python.org/downloads/"
        .into())
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
///
/// Uses a lightweight marker file inside the venv to avoid spawning a
/// Python subprocess on every launch.  The marker stores the venv
/// binary's mtime — if the mtime matches, the version is still valid.
pub async fn venv_is_valid(venv_dir: &Path) -> bool {
    let py = venv_python(venv_dir);
    if !py.exists() {
        return false;
    }

    // Fast path: marker mtime matches the Python binary's mtime.
    let marker_path = venv_dir.join(VENV_MARKER);
    if let (Ok(py_meta), Ok(marker_contents)) = (
        std::fs::metadata(&py),
        std::fs::read_to_string(&marker_path),
    ) {
        if let Ok(py_mtime) = py_meta.modified() {
            let mtime_secs = py_mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if marker_contents.trim() == mtime_secs.to_string() {
                return true;
            }
        }
    }

    // Slow path: run a Python version-check subprocess.
    let valid = check_python_version(py.to_string_lossy().as_ref()).await;

    // Write marker so the next launch can skip the subprocess.
    if valid {
        write_venv_marker(venv_dir, &py);
    }

    valid
}

/// Write the venv validation marker with the Python binary's mtime.
fn write_venv_marker(venv_dir: &Path, py: &Path) {
    if let Ok(meta) = std::fs::metadata(py) {
        if let Ok(mtime) = meta.modified() {
            let mtime_secs = mtime
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = std::fs::write(venv_dir.join(VENV_MARKER), mtime_secs.to_string());
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────

/// Resolve a bare executable name to the real absolute path Python reports
/// for itself via `sys.executable`. This bypasses issues with WindowsApps
/// app execution aliases that `where.exe` returns — those short aliases
/// cannot be spawned reliably from non-shell contexts like Tauri desktop apps,
/// but they *can* be executed by the shell that `Command` inherits, so we use
/// the alias just long enough to ask Python where it really lives.
#[cfg(target_os = "windows")]
async fn resolve_real_python_path(name: &str) -> Option<String> {
    let output = Command::new(name)
        .args(["-c", "import sys; print(sys.executable)"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    // Verify the resolved path actually exists on disk
    if Path::new(&path).exists() {
        Some(path)
    } else {
        None
    }
}

async fn binary_exists(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where.exe")
            .arg(name)
            .creation_flags(CREATE_NO_WINDOW)
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
    let mut cmd = Command::new(executable);
    cmd.args([
        "-c",
        "import sys; exit(0 if sys.version_info >= (3,12) else 1)",
    ])
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.status().await.map(|s| s.success()).unwrap_or(false)
}
