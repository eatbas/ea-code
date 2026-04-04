use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if let Err(error) = ensure_symphony_checkout() {
        panic!("{error}");
    }

    tauri_build::build()
}

fn ensure_symphony_checkout() -> Result<(), String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .ok_or_else(|| "Cannot determine repository root for symphony resources".to_string())?;

    let symphony_dir = repo_root.join("symphony-api");
    let gitmodules = repo_root.join(".gitmodules");

    println!("cargo:rerun-if-changed={}", gitmodules.display());
    println!(
        "cargo:rerun-if-changed={}",
        symphony_dir.join("pyproject.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        symphony_dir.join("src").display()
    );

    if symphony_dir_has_source(&symphony_dir) {
        return Ok(());
    }

    if !gitmodules.exists() {
        return Err(format!(
            "Missing symphony source at {} and no .gitmodules file was found at {}.",
            symphony_dir.display(),
            gitmodules.display()
        ));
    }

    println!("cargo:warning=Initialising symphony submodule for Tauri resources");

    let status = Command::new("git")
        .args([
            "submodule",
            "update",
            "--init",
            "--recursive",
            "symphony-api",
        ])
        .current_dir(repo_root)
        .status()
        .map_err(|error| format!("Failed to initialise symphony submodule: {error}"))?;

    if status.success() && symphony_dir_has_source(&symphony_dir) {
        return Ok(());
    }

    Err(format!(
        "symphony source is missing at {}. Run `git submodule update --init --recursive` from the repository root.",
        symphony_dir.display()
    ))
}

fn symphony_dir_has_source(dir: &Path) -> bool {
    dir.join("pyproject.toml").exists() && dir.join("src").is_dir()
}
