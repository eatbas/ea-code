use std::collections::HashMap;

use diesel::prelude::*;

use crate::db::DbPool;
use crate::schema::project_settings;

use super::models::NewProjectSetting;

/// Returns all overrides for a project keyed by setting key.
pub fn get_map_for_project(pool: &DbPool, project_id: i32) -> Result<HashMap<String, String>, String> {
    let mut conn = super::get_conn(pool)?;

    let rows = project_settings::table
        .filter(project_settings::project_id.eq(project_id))
        .select((project_settings::setting_key, project_settings::setting_value))
        .load::<(String, String)>(&mut conn)
        .map_err(|e| format!("Failed to load project settings: {e}"))?;

    Ok(rows.into_iter().collect())
}

/// Replaces all project overrides in one transaction.
pub fn replace_for_project(
    pool: &DbPool,
    project_id: i32,
    entries: &[(String, String)],
) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    conn.transaction::<(), diesel::result::Error, _>(|conn| {
        diesel::delete(
            project_settings::table.filter(project_settings::project_id.eq(project_id)),
        )
        .execute(conn)?;

        if !entries.is_empty() {
            let records = entries
                .iter()
                .map(|(key, value)| NewProjectSetting {
                    project_id,
                    setting_key: key.as_str(),
                    setting_value: value.as_str(),
                })
                .collect::<Vec<_>>();

            diesel::insert_into(project_settings::table)
                .values(&records)
                .execute(conn)?;
        }

        Ok(())
    })
    .map_err(|e| format!("Failed to replace project settings: {e}"))
}

/// Clears all overrides for a project.
pub fn clear_for_project(pool: &DbPool, project_id: i32) -> Result<(), String> {
    replace_for_project(pool, project_id, &[])
}
