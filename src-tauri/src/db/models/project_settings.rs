use diesel::prelude::*;
use serde::Serialize;

use crate::schema::project_settings;

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = project_settings)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSettingRow {
    pub id: i32,
    pub project_id: i32,
    pub setting_key: String,
    pub setting_value: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = project_settings)]
pub struct NewProjectSetting<'a> {
    pub project_id: i32,
    pub setting_key: &'a str,
    pub setting_value: &'a str,
}
