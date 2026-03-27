use std::path::{Path, PathBuf};

/// Returns `true` if the directory looks like an initialised hive-api checkout
/// (i.e. contains `pyproject.toml`).
pub fn hive_dir_has_source(dir: &Path) -> bool {
    dir.join("pyproject.toml").exists()
}

/// Locate the hive-api directory relative to the project root.
///
/// In development, this is `{repo_root}/hive-api/`.
/// In a bundled release, it would be inside the Tauri resource directory.
///
/// If the directory exists but the git submodule is not initialised (no
/// `pyproject.toml`), attempts `git submodule update --init` automatically.
pub fn find_hive_dir() -> Result<PathBuf, String> {
    // Development: walk up from src-tauri to find repo root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = frontend/desktop/src-tauri
    // repo root = manifest_dir / ../../..
    let repo_root = manifest_dir
        .parent() // frontend/desktop
        .and_then(|p| p.parent()) // frontend
        .and_then(|p| p.parent()) // repo root
        .ok_or_else(|| "Cannot determine repository root".to_string())?;

    let hive_dir = repo_root.join("hive-api");
    if hive_dir.is_dir() {
        if hive_dir_has_source(&hive_dir) {
            return Ok(hive_dir);
        }

        // Directory exists but source is missing — try initialising the submodule.
        eprintln!("[sidecar] hive-api directory exists but source is missing — running git submodule update --init");
        let status = std::process::Command::new("git")
            .args(["submodule", "update", "--init", "hive-api"])
            .current_dir(repo_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status();

        match status {
            Ok(s) if s.success() && hive_dir_has_source(&hive_dir) => {
                eprintln!("[sidecar] hive-api submodule initialised successfully");
                return Ok(hive_dir);
            }
            Ok(s) => {
                eprintln!("[sidecar] git submodule update --init exited with {s}");
            }
            Err(e) => {
                eprintln!("[sidecar] Failed to run git submodule update: {e}");
            }
        }

        return Err("hive-api directory exists but has no source code. \
             Run `git submodule update --init` from the repository root."
            .into());
    }

    // Bundled: check platform-specific resource locations next to the executable.
    if let Ok(exe) = std::env::current_exe() {
        // Follow symlinks to get the real executable path (macOS aliases, etc.)
        let exe = exe.canonicalize().unwrap_or(exe);
        if let Some(exe_dir) = exe.parent() {
            // Windows: resources sit next to the exe
            let bundled = exe_dir.join("hive-api");
            if bundled.is_dir() && hive_dir_has_source(&bundled) {
                return Ok(bundled);
            }

            // macOS: resources are at Contents/Resources/ (exe is at Contents/MacOS/)
            #[cfg(target_os = "macos")]
            if let Some(contents_dir) = exe_dir.parent() {
                let mac_resources = contents_dir.join("Resources").join("hive-api");
                if mac_resources.is_dir() && hive_dir_has_source(&mac_resources) {
                    return Ok(mac_resources);
                }
            }
        }
    }

    Err("hive-api directory not found. Ensure the git submodule is initialised.".into())
}
