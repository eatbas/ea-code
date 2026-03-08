use tauri::State;

use crate::db;
use crate::models::Skill;

use super::AppState;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillPayload {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tags: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSkillPayload {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub tags: Option<String>,
    pub is_active: Option<bool>,
}

fn to_skill(row: db::models::SkillRow) -> Skill {
    Skill {
        id: row.id,
        name: row.name,
        description: row.description,
        instructions: row.instructions,
        tags: row.tags,
        is_active: row.is_active,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn clean(text: &str) -> String {
    text.trim().to_string()
}

/// Lists all skills.
#[tauri::command]
pub async fn list_skills(state: State<'_, AppState>) -> Result<Vec<Skill>, String> {
    let rows = db::skills::list(&state.db)?;
    Ok(rows.into_iter().map(to_skill).collect())
}

/// Gets a single skill by ID.
#[tauri::command]
pub async fn get_skill(state: State<'_, AppState>, id: String) -> Result<Skill, String> {
    let row = db::skills::get_by_id(&state.db, id.trim())?
        .ok_or_else(|| "Skill not found".to_string())?;
    Ok(to_skill(row))
}

/// Creates a skill.
#[tauri::command]
pub async fn create_skill(
    state: State<'_, AppState>,
    payload: CreateSkillPayload,
) -> Result<Skill, String> {
    let name = clean(&payload.name);
    if name.is_empty() {
        return Err("Skill name is required".to_string());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let description = clean(&payload.description);
    let instructions = payload.instructions.trim().replace("\r\n", "\n");
    let tags = payload.tags.unwrap_or_default().trim().to_string();
    let is_active = payload.is_active.unwrap_or(true);

    let row = db::skills::create(
        &state.db,
        &db::models::NewSkill {
            id: &id,
            name: &name,
            description: &description,
            instructions: &instructions,
            tags: &tags,
            is_active,
        },
    )?;

    Ok(to_skill(row))
}

/// Updates an existing skill.
#[tauri::command]
pub async fn update_skill(
    state: State<'_, AppState>,
    payload: UpdateSkillPayload,
) -> Result<Skill, String> {
    let id = payload.id.trim().to_string();
    if id.is_empty() {
        return Err("Skill ID is required".to_string());
    }

    let existing = db::skills::get_by_id(&state.db, &id)?
        .ok_or_else(|| "Skill not found".to_string())?;

    let name = payload.name.as_deref().map(clean).unwrap_or(existing.name);
    if name.is_empty() {
        return Err("Skill name is required".to_string());
    }

    let description = payload
        .description
        .as_deref()
        .map(clean)
        .unwrap_or(existing.description);
    let instructions = payload
        .instructions
        .as_deref()
        .map(|text| text.trim().replace("\r\n", "\n"))
        .unwrap_or(existing.instructions);
    let tags = payload.tags.as_deref().map(clean).unwrap_or(existing.tags);
    let is_active = payload.is_active.unwrap_or(existing.is_active);

    let now = chrono::Utc::now().to_rfc3339();
    let row = db::skills::update(
        &state.db,
        &id,
        &db::models::SkillChangeset {
            name: &name,
            description: &description,
            instructions: &instructions,
            tags: &tags,
            is_active,
            updated_at: &now,
        },
    )?;

    Ok(to_skill(row))
}

/// Deletes a skill.
#[tauri::command]
pub async fn delete_skill(state: State<'_, AppState>, id: String) -> Result<(), String> {
    db::skills::delete(&state.db, id.trim())
}
