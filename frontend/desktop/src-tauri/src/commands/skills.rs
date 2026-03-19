use crate::models::SkillFile;
use crate::storage;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillPayload {
    pub name: String,
    pub description: String,
    pub instructions: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSkillPayload {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
}

fn clean(text: &str) -> String {
    text.trim().to_string()
}

/// Lists all skills.
#[tauri::command]
pub async fn list_skills() -> Result<Vec<SkillFile>, String> {
    storage::skills::list_skills()
}

/// Gets a single skill by ID.
#[tauri::command]
pub async fn get_skill(id: String) -> Result<SkillFile, String> {
    storage::skills::get_skill(id.trim())
}

/// Creates a skill.
#[tauri::command]
pub async fn create_skill(payload: CreateSkillPayload) -> Result<SkillFile, String> {
    let name = clean(&payload.name);
    if name.is_empty() {
        return Err("Skill name is required".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let description = clean(&payload.description);
    let instructions = payload.instructions.trim().replace("\r\n", "\n");

    let skill = storage::skills::create_skill(id, name, description, instructions);

    storage::skills::save_skill(&skill)?;
    Ok(skill)
}

/// Updates an existing skill.
#[tauri::command]
pub async fn update_skill(payload: UpdateSkillPayload) -> Result<SkillFile, String> {
    let id = payload.id.trim().to_string();
    if id.is_empty() {
        return Err("Skill ID is required".to_string());
    }

    let mut skill = storage::skills::get_skill(&id)?;

    if let Some(name) = payload.name {
        let name = clean(&name);
        if name.is_empty() {
            return Err("Skill name is required".to_string());
        }
        skill.name = name;
    }

    if let Some(description) = payload.description {
        skill.description = clean(&description);
    }

    if let Some(instructions) = payload.instructions {
        skill.prompt = instructions.trim().replace("\r\n", "\n");
    }

    skill.updated_at = storage::now_rfc3339();
    storage::skills::save_skill(&skill)?;
    Ok(skill)
}

/// Deletes a skill.
#[tauri::command]
pub async fn delete_skill(id: String) -> Result<(), String> {
    storage::skills::delete_skill(id.trim())
}
