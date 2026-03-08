use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::skills;

use super::models::{NewSkill, SkillChangeset, SkillRow};

/// Lists all skills ordered by name.
pub fn list(pool: &DbPool) -> Result<Vec<SkillRow>, String> {
    let mut conn = super::get_conn(pool)?;

    skills::table
        .order(skills::name.asc())
        .load::<SkillRow>(&mut conn)
        .map_err(|e| format!("Failed to list skills: {e}"))
}

/// Lists active skills only, ordered by most recently updated.
pub fn list_active(pool: &DbPool) -> Result<Vec<SkillRow>, String> {
    let mut conn = super::get_conn(pool)?;

    skills::table
        .filter(skills::is_active.eq(true))
        .order(skills::updated_at.desc())
        .load::<SkillRow>(&mut conn)
        .map_err(|e| format!("Failed to list active skills: {e}"))
}

/// Fetches a skill by ID.
pub fn get_by_id(pool: &DbPool, id: &str) -> Result<Option<SkillRow>, String> {
    let mut conn = super::get_conn(pool)?;

    skills::table
        .find(id)
        .first::<SkillRow>(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to load skill: {e}"))
}

/// Creates a new skill and returns it.
pub fn create(pool: &DbPool, new_skill: &NewSkill<'_>) -> Result<SkillRow, String> {
    let mut conn = super::get_conn(pool)?;

    diesel::insert_into(skills::table)
        .values(new_skill)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to create skill: {e}"))?;

    skills::table
        .find(new_skill.id)
        .first::<SkillRow>(&mut conn)
        .map_err(|e| format!("Failed to load created skill: {e}"))
}

/// Updates an existing skill and returns the fresh row.
pub fn update(pool: &DbPool, id: &str, changeset: &SkillChangeset<'_>) -> Result<SkillRow, String> {
    let mut conn = super::get_conn(pool)?;

    let affected = diesel::update(skills::table.find(id))
        .set(changeset)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update skill: {e}"))?;

    if affected == 0 {
        return Err("Skill not found".to_string());
    }

    skills::table
        .find(id)
        .first::<SkillRow>(&mut conn)
        .map_err(|e| format!("Failed to load updated skill: {e}"))
}

/// Deletes a skill by ID.
pub fn delete(pool: &DbPool, id: &str) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    let affected = diesel::delete(skills::table.find(id))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete skill: {e}"))?;

    if affected == 0 {
        return Err("Skill not found".to_string());
    }

    Ok(())
}
