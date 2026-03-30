use std::path::Path;

/// Traverse all run directories across every known workspace.
///
/// Delegates to [`traverse_sessions_and_runs`] with a no-op session
/// callback. See that function for detailed parameter documentation.
pub fn traverse_runs<F>(callback: &mut F) -> Result<(), String>
where
    F: FnMut(&Path, &str, &str, &str) -> Result<(), String>,
{
    traverse_sessions_and_runs(&mut |_, _, _| Ok(()), callback)
}

/// Traverse all sessions and runs across every known workspace.
///
/// Like [`traverse_runs`], but also calls `on_session` once per session
/// directory before descending into its runs. Useful for gathering
/// session-level statistics.
pub fn traverse_sessions_and_runs<S, R>(on_session: &mut S, on_run: &mut R) -> Result<(), String>
where
    S: FnMut(&Path, &str, &str) -> Result<(), String>,
    R: FnMut(&Path, &str, &str, &str) -> Result<(), String>,
{
    let projects = crate::storage::projects::list_projects(true).unwrap_or_default();

    for project in &projects {
        let workspace = Path::new(&project.path);
        if !workspace.exists() {
            continue;
        }

        let sessions_dir = workspace.join(".ea-code").join("sessions");
        if !sessions_dir.is_dir() {
            continue;
        }

        let session_entries = match std::fs::read_dir(&sessions_dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!(
                    "Warning: could not read sessions directory {}: {e}",
                    sessions_dir.display()
                );
                continue;
            }
        };

        for session_entry in session_entries {
            let session_entry = match session_entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let session_path = session_entry.path();

            if !session_path.is_dir() {
                continue;
            }

            let session_id = match session_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            on_session(&session_path, &project.path, &session_id)?;

            let runs_dir = session_path.join("runs");
            if !runs_dir.is_dir() {
                continue;
            }

            let run_entries = match std::fs::read_dir(&runs_dir) {
                Ok(entries) => entries,
                Err(e) => {
                    eprintln!(
                        "Warning: could not read runs directory {}: {e}",
                        runs_dir.display()
                    );
                    continue;
                }
            };

            for run_entry in run_entries {
                let run_entry = match run_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let run_path = run_entry.path();

                if !run_path.is_dir() {
                    continue;
                }

                let run_id = match run_path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                on_run(&run_path, &project.path, &session_id, &run_id)?;
            }
        }
    }

    Ok(())
}
