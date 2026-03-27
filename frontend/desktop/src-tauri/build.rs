use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = ensure_hive_api_checkout() {
        panic!("{error}");
    }

    tauri_build::build()
}

fn ensure_hive_api_checkout() -> Result<(), String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .ok_or_else(|| "Cannot determine repository root for hive-api resources".to_string())?;

    let hive_dir = repo_root.join("hive-api");
    let gitmodules = repo_root.join(".gitmodules");

    println!("cargo:rerun-if-changed={}", gitmodules.display());
    println!(
        "cargo:rerun-if-changed={}",
        hive_dir.join("pyproject.toml").display()
    );
    println!("cargo:rerun-if-changed={}", hive_dir.join("src").display());

    if hive_dir_has_source(&hive_dir) {
        return Ok(());
    }

    if !gitmodules.exists() {
        return Err(format!(
            "Missing hive-api source at {} and no .gitmodules file was found at {}.",
            hive_dir.display(),
            gitmodules.display()
        ));
    }

    println!("cargo:warning=Initialising hive-api submodule for Tauri resources");

    let status = Command::new("git")
        .args(["submodule", "update", "--init", "--recursive", "hive-api"])
        .current_dir(repo_root)
        .status()
        .map_err(|error| format!("Failed to initialise hive-api submodule: {error}"))?;

    if status.success() && hive_dir_has_source(&hive_dir) {
        return Ok(());
    }

    Err(format!(
        "hive-api source is missing at {}. Run `git submodule update --init --recursive` from the repository root.",
        hive_dir.display()
    ))
}

fn hive_dir_has_source(dir: &Path) -> bool {
    dir.join("pyproject.toml").exists() && dir.join("src").is_dir()
}
