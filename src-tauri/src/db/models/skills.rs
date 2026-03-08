use diesel::prelude::*;
use serde::Serialize;

use crate::schema::skills;

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = skills)]
#[serde(rename_all = "camelCase")]
pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tags: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = skills)]
pub struct NewSkill<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    pub instructions: &'a str,
    pub tags: &'a str,
    pub is_active: bool,
}

#[derive(AsChangeset)]
#[diesel(table_name = skills)]
pub struct SkillChangeset<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub instructions: &'a str,
    pub tags: &'a str,
    pub is_active: bool,
    pub updated_at: &'a str,
}
