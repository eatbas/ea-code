use crate::models::SkillFile;

use super::{atomic_write, config_dir, now_rfc3339, validate_id, with_skills_lock};

/// Returns the path to a skill file.
fn skill_path(id: &str) -> Result<std::path::PathBuf, String> {
    validate_id(id)?;
    Ok(config_dir()?.join("skills").join(format!("{id}.json")))
}

/// Lists all skills by globbing skills/*.json.
///
/// Note: Sorting relies on RFC 3339 timestamp format (e.g., "2026-03-11T14:30:00Z").
/// This format sorts lexicographically for timestamps in the same timezone.
pub fn list_skills() -> Result<Vec<SkillFile>, String> {
    let skills_dir = config_dir()?.join("skills");

    if !skills_dir.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();

    let entries = std::fs::read_dir(&skills_dir)
        .map_err(|e| format!("Failed to read skills directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<SkillFile>(&contents) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => eprintln!(
                        "Warning: Failed to parse skill file {}: {e}",
                        path.display()
                    ),
                },
                Err(e) => eprintln!("Warning: Failed to read skill file {}: {e}", path.display()),
            }
        }
    }

    // Sort by updated_at descending (most recently updated first)
    // Note: RFC 3339 timestamps sort lexicographically when in the same timezone
    skills.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(skills)
}

/// Gets a single skill by ID.
pub fn get_skill(id: &str) -> Result<SkillFile, String> {
    validate_id(id)?;
    let path = skill_path(id)?;

    if !path.exists() {
        return Err(format!("Skill not found: {id}"));
    }

    let contents =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read skill file: {e}"))?;

    let skill: SkillFile =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse skill file: {e}"))?;

    Ok(skill)
}

/// Saves a skill (creates or updates).
/// H8: Protected by file lock for concurrent access.
pub fn save_skill(skill: &SkillFile) -> Result<(), String> {
    validate_id(&skill.id)?;
    with_skills_lock(|| {
        let path = skill_path(&skill.id)?;

        let json = serde_json::to_string_pretty(skill)
            .map_err(|e| format!("Failed to serialise skill: {e}"))?;

        atomic_write(&path, &json)
    })
}

/// Deletes a skill by ID.
/// H8: Protected by file lock for concurrent access.
pub fn delete_skill(id: &str) -> Result<(), String> {
    validate_id(id)?;
    with_skills_lock(|| {
        let path = skill_path(id)?;

        if !path.exists() {
            return Err(format!("Skill not found: {id}"));
        }

        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete skill file: {e}"))?;

        Ok(())
    })
}

/// Creates a new skill with the current timestamp.
pub fn create_skill(id: String, name: String, description: String, prompt: String) -> SkillFile {
    let now = now_rfc3339();
    SkillFile {
        id,
        name,
        description,
        prompt,
        tags: Vec::new(),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    }
}
