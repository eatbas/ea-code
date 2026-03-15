use serde::{Deserialize, Serialize};

/// Frontend-facing skill record.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tags: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}
