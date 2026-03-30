use std::path::{Path, PathBuf};

/// Returns `true` if the directory looks like an initialised symphony checkout
/// (i.e. contains `pyproject.toml`).
pub fn symphony_dir_has_source(dir: &Path) -> bool {
    dir.join("pyproject.toml").exists()
}

/// Locate the symphony directory relative to the project root.
///
/// In development, this is `{repo_root}/symphony/`.
/// In a bundled release, it would be inside the Tauri resource directory.
///
/// If the directory exists but the git submodule is not initialised (no
/// `pyproject.toml`), attempts `git submodule update --init` automatically.
pub fn find_symphony_dir() -> Result<PathBuf, String> {
    // Development: walk up from src-tauri to find repo root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // manifest_dir = frontend/desktop/src-tauri
    // repo root = manifest_dir / ../../..
    let repo_root = manifest_dir
        .parent() // frontend/desktop
        .and_then(|p| p.parent()) // frontend
        .and_then(|p| p.parent()) // repo root
        .ok_or_else(|| "Cannot determine repository root".to_string())?;

    let symphony_dir = repo_root.join("symphony");
    if symphony_dir.is_dir() {
        if symphony_dir_has_source(&symphony_dir) {
            return Ok(symphony_dir);
        }

        // Directory exists but source is missing — try initialising the submodule.
        eprintln!("[sidecar] symphony directory exists but source is missing — running git submodule update --init");
        let status = std::process::Command::new("git")
            .args(["submodule", "update", "--init", "symphony"])
            .current_dir(repo_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status();

        match status {
            Ok(s) if s.success() && symphony_dir_has_source(&symphony_dir) => {
                eprintln!("[sidecar] symphony submodule initialised successfully");
                return Ok(symphony_dir);
            }
            Ok(s) => {
                eprintln!("[sidecar] git submodule update --init exited with {s}");
            }
            Err(e) => {
                eprintln!("[sidecar] Failed to run git submodule update: {e}");
            }
        }

        return Err("symphony directory exists but has no source code. \
             Run `git submodule update --init` from the repository root."
            .into());
    }

    // Bundled: check platform-specific resource locations next to the executable.
    if let Ok(exe) = std::env::current_exe() {
        // Follow symlinks to get the real executable path (macOS aliases, etc.)
        let exe = exe.canonicalize().unwrap_or(exe);
        if let Some(exe_dir) = exe.parent() {
            // Windows: resources sit next to the exe
            let bundled = exe_dir.join("symphony");
            if bundled.is_dir() && symphony_dir_has_source(&bundled) {
                return Ok(bundled);
            }

            // macOS: resources are at Contents/Resources/ (exe is at Contents/MacOS/)
            #[cfg(target_os = "macos")]
            if let Some(contents_dir) = exe_dir.parent() {
                let mac_resources = contents_dir.join("Resources").join("symphony");
                if mac_resources.is_dir() && symphony_dir_has_source(&mac_resources) {
                    return Ok(mac_resources);
                }
            }
        }
    }

    Err("symphony directory not found. Ensure the git submodule is initialised.".into())
}
