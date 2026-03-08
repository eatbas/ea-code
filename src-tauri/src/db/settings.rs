use diesel::prelude::*;

use crate::db::DbPool;
use crate::models::AppSettings;
use crate::schema::settings;

use super::models::{SettingsChangeset, SettingsRow};

/// Loads settings from the database (single row, id = 1).
pub fn get(pool: &DbPool) -> Result<AppSettings, String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    let row: SettingsRow = settings::table
        .find(1)
        .first(&mut conn)
        .map_err(|e| format!("Failed to load settings: {e}"))?;

    Ok(row_to_app_settings(&row))
}

/// Persists settings to the database (updates the single row).
pub fn update(pool: &DbPool, s: &AppSettings) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    let changeset = SettingsChangeset {
        claude_path: s.claude_path.clone(),
        codex_path: s.codex_path.clone(),
        gemini_path: s.gemini_path.clone(),
        generator_agent: backend_to_str(&s.generator_agent),
        reviewer_agent: backend_to_str(&s.reviewer_agent),
        fixer_agent: backend_to_str(&s.fixer_agent),
        final_judge_agent: backend_to_str(&s.final_judge_agent),
        max_iterations: s.max_iterations as i32,
        require_git: s.require_git,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    diesel::update(settings::table.find(1))
        .set(&changeset)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update settings: {e}"))?;

    Ok(())
}

/// Converts a `SettingsRow` to the application-facing `AppSettings`.
fn row_to_app_settings(row: &SettingsRow) -> AppSettings {
    use crate::models::AgentBackend;

    fn parse_backend(s: &str) -> AgentBackend {
        match s {
            "codex" => AgentBackend::Codex,
            "gemini" => AgentBackend::Gemini,
            _ => AgentBackend::Claude,
        }
    }

    AppSettings {
        claude_path: row.claude_path.clone(),
        codex_path: row.codex_path.clone(),
        gemini_path: row.gemini_path.clone(),
        generator_agent: parse_backend(&row.generator_agent),
        reviewer_agent: parse_backend(&row.reviewer_agent),
        fixer_agent: parse_backend(&row.fixer_agent),
        final_judge_agent: parse_backend(&row.final_judge_agent),
        max_iterations: row.max_iterations as u32,
        require_git: row.require_git,
    }
}

/// Converts an `AgentBackend` to its database string representation.
fn backend_to_str(b: &crate::models::AgentBackend) -> String {
    match b {
        crate::models::AgentBackend::Claude => "claude".to_string(),
        crate::models::AgentBackend::Codex => "codex".to_string(),
        crate::models::AgentBackend::Gemini => "gemini".to_string(),
    }
}
