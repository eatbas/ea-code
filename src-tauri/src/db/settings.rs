use diesel::prelude::*;
use serde_json::{Map, Value};

use crate::db::DbPool;
use crate::models::AppSettings;
use crate::schema::settings;

use super::models::{SettingsChangeset, SettingsRow};

/// Loads settings from the database (single row, id = 1).
pub fn get(pool: &DbPool) -> Result<AppSettings, String> {
    let mut conn = super::get_conn(pool)?;

    let row: SettingsRow = settings::table
        .find(1)
        .first(&mut conn)
        .map_err(|e| format!("Failed to load settings: {e}"))?;

    Ok(row_to_app_settings(&row))
}

/// Persists settings to the database (updates the single row).
pub fn update(pool: &DbPool, s: &AppSettings) -> Result<(), String> {
    let mut conn = super::get_conn(pool)?;

    let changeset = SettingsChangeset {
        claude_path: s.claude_path.clone(),
        codex_path: s.codex_path.clone(),
        gemini_path: s.gemini_path.clone(),
        kimi_path: s.kimi_path.clone(),
        opencode_path: s.opencode_path.clone(),
        copilot_path: s.copilot_path.clone(),
        prompt_enhancer_agent: backend_to_str(&s.prompt_enhancer_agent),
        skill_selector_agent: backend_to_opt(s.skill_selector_agent.as_ref()),
        planner_agent: backend_to_opt(s.planner_agent.as_ref()),
        plan_auditor_agent: backend_to_opt(s.plan_auditor_agent.as_ref()),
        generator_agent: backend_to_str(&s.generator_agent),
        reviewer_agent: backend_to_str(&s.reviewer_agent),
        fixer_agent: backend_to_str(&s.fixer_agent),
        final_judge_agent: backend_to_str(&s.final_judge_agent),
        executive_summary_agent: backend_to_str(&s.executive_summary_agent),
        max_iterations: s.max_iterations as i32,
        require_git: s.require_git,
        updated_at: chrono::Utc::now().to_rfc3339(),
        claude_model: s.claude_model.clone(),
        codex_model: s.codex_model.clone(),
        gemini_model: s.gemini_model.clone(),
        kimi_model: s.kimi_model.clone(),
        opencode_model: s.opencode_model.clone(),
        copilot_model: s.copilot_model.clone(),
        prompt_enhancer_model: s.prompt_enhancer_model.clone(),
        skill_selector_model: s.skill_selector_model.clone(),
        planner_model: s.planner_model.clone(),
        plan_auditor_model: s.plan_auditor_model.clone(),
        generator_model: s.generator_model.clone(),
        reviewer_model: s.reviewer_model.clone(),
        fixer_model: s.fixer_model.clone(),
        final_judge_model: s.final_judge_model.clone(),
        executive_summary_model: s.executive_summary_model.clone(),
        require_plan_approval: s.require_plan_approval,
        plan_auto_approve_timeout_sec: s.plan_auto_approve_timeout_sec as i32,
        max_plan_revisions: s.max_plan_revisions as i32,
        token_optimized_prompts: s.token_optimized_prompts,
        agent_retry_count: s.agent_retry_count as i32,
        agent_timeout_ms: s.agent_timeout_ms as i32,
        agent_max_turns: s.agent_max_turns as i32,
        mode: s.mode.clone(),
        update_cli_on_run: s.update_cli_on_run,
        fail_on_cli_update_error: s.fail_on_cli_update_error,
        cli_update_timeout_ms: s.cli_update_timeout_ms as i32,
        skill_selection_mode: s.skill_selection_mode.clone(),
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
            "claude" => AgentBackend::Claude,
            "codex" => AgentBackend::Codex,
            "gemini" => AgentBackend::Gemini,
            "kimi" => AgentBackend::Kimi,
            // Legacy compatibility: Copilot assignments are migrated to Codex.
            "copilot" => AgentBackend::Codex,
            "opencode" => AgentBackend::OpenCode,
            _ => {
                eprintln!("Unknown backend in settings row: {s}; defaulting to claude");
                AgentBackend::Claude
            }
        }
    }

    fn parse_optional_backend(s: Option<&str>) -> Option<AgentBackend> {
        s.map(parse_backend)
    }

    AppSettings {
        claude_path: row.claude_path.clone(),
        codex_path: row.codex_path.clone(),
        gemini_path: row.gemini_path.clone(),
        kimi_path: row.kimi_path.clone(),
        opencode_path: row.opencode_path.clone(),
        copilot_path: row.copilot_path.clone(),
        prompt_enhancer_agent: parse_backend(&row.prompt_enhancer_agent),
        skill_selector_agent: parse_optional_backend(row.skill_selector_agent.as_deref()),
        planner_agent: parse_optional_backend(row.planner_agent.as_deref()),
        plan_auditor_agent: parse_optional_backend(row.plan_auditor_agent.as_deref()),
        generator_agent: parse_backend(&row.generator_agent),
        reviewer_agent: parse_backend(&row.reviewer_agent),
        fixer_agent: parse_backend(&row.fixer_agent),
        final_judge_agent: parse_backend(&row.final_judge_agent),
        executive_summary_agent: parse_backend(&row.executive_summary_agent),
        max_iterations: row.max_iterations as u32,
        require_git: row.require_git,
        claude_model: row.claude_model.clone(),
        codex_model: row.codex_model.clone(),
        gemini_model: row.gemini_model.clone(),
        kimi_model: row.kimi_model.clone(),
        opencode_model: row.opencode_model.clone(),
        copilot_model: row.copilot_model.clone(),
        prompt_enhancer_model: row.prompt_enhancer_model.clone(),
        skill_selector_model: row.skill_selector_model.clone(),
        planner_model: row.planner_model.clone(),
        plan_auditor_model: row.plan_auditor_model.clone(),
        generator_model: row.generator_model.clone(),
        reviewer_model: row.reviewer_model.clone(),
        fixer_model: row.fixer_model.clone(),
        final_judge_model: row.final_judge_model.clone(),
        executive_summary_model: row.executive_summary_model.clone(),
        require_plan_approval: row.require_plan_approval,
        plan_auto_approve_timeout_sec: row.plan_auto_approve_timeout_sec as u32,
        max_plan_revisions: row.max_plan_revisions as u32,
        token_optimized_prompts: row.token_optimized_prompts,
        agent_retry_count: row.agent_retry_count as u32,
        agent_timeout_ms: row.agent_timeout_ms as u64,
        agent_max_turns: row.agent_max_turns as u32,
        mode: row.mode.clone(),
        update_cli_on_run: row.update_cli_on_run,
        fail_on_cli_update_error: row.fail_on_cli_update_error,
        cli_update_timeout_ms: row.cli_update_timeout_ms as u64,
        skill_selection_mode: row.skill_selection_mode.clone(),
    }
}

/// Converts an `AgentBackend` to its database string representation.
fn backend_to_str(b: &crate::models::AgentBackend) -> String {
    match b {
        crate::models::AgentBackend::Claude => "claude".to_string(),
        crate::models::AgentBackend::Codex => "codex".to_string(),
        crate::models::AgentBackend::Gemini => "gemini".to_string(),
        crate::models::AgentBackend::Kimi => "kimi".to_string(),
        crate::models::AgentBackend::OpenCode => "opencode".to_string(),
    }
}

/// Converts an optional `AgentBackend` to its nullable DB representation.
fn backend_to_opt(b: Option<&crate::models::AgentBackend>) -> Option<String> {
    b.map(backend_to_str)
}

/// Loads merged settings for a workspace path (global + project overrides).
pub fn get_merged_for_workspace(pool: &DbPool, workspace_path: &str) -> Result<AppSettings, String> {
    let base = get(pool)?;
    let maybe_project = super::projects::get_by_path(pool, workspace_path)?;
    let Some(project) = maybe_project else {
        return Ok(base);
    };

    let overrides = super::project_settings::get_map_for_project(pool, project.id)?;
    merge_settings(base, &overrides)
}

/// Saves project-specific overrides for a workspace by diffing against global settings.
pub fn save_project_overrides_for_workspace(
    pool: &DbPool,
    workspace_path: &str,
    merged_settings: &AppSettings,
) -> Result<(), String> {
    let base = get(pool)?;
    let project = ensure_project(pool, workspace_path)?;

    let base_obj = app_settings_to_object(&base)?;
    let merged_obj = app_settings_to_object(merged_settings)?;

    let mut diff_entries = Vec::new();
    for (key, merged_val) in merged_obj {
        let base_val = base_obj.get(&key);
        if base_val == Some(&merged_val) {
            continue;
        }
        let stored = serde_json::to_string(&merged_val)
            .map_err(|e| format!("Failed to serialise project setting value for {key}: {e}"))?;
        diff_entries.push((key, stored));
    }

    super::project_settings::replace_for_project(pool, project.id, &diff_entries)
}

/// Clears project-specific overrides for a workspace.
pub fn clear_project_overrides_for_workspace(pool: &DbPool, workspace_path: &str) -> Result<(), String> {
    if let Some(project) = super::projects::get_by_path(pool, workspace_path)? {
        super::project_settings::clear_for_project(pool, project.id)?;
    }
    Ok(())
}

fn ensure_project(pool: &DbPool, workspace_path: &str) -> Result<super::models::ProjectRow, String> {
    if let Some(project) = super::projects::get_by_path(pool, workspace_path)? {
        return Ok(project);
    }

    let workspace_name = std::path::Path::new(workspace_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| workspace_path.to_string());
    let ws_info = crate::git::workspace_info(workspace_path);
    let project_id = super::projects::upsert(
        pool,
        workspace_path,
        &workspace_name,
        ws_info.is_git_repo,
        ws_info.branch.as_deref(),
    )?;

    super::projects::get_by_path(pool, workspace_path)?
        .filter(|p| p.id == project_id)
        .ok_or_else(|| "Failed to load project after upsert".to_string())
}

fn app_settings_to_object(settings: &AppSettings) -> Result<Map<String, Value>, String> {
    serde_json::to_value(settings)
        .map_err(|e| format!("Failed to serialise settings: {e}"))?
        .as_object()
        .cloned()
        .ok_or_else(|| "Settings value is not an object".to_string())
}

fn merge_settings(base: AppSettings, overrides: &std::collections::HashMap<String, String>) -> Result<AppSettings, String> {
    let mut object = app_settings_to_object(&base)?;

    for (key, value_raw) in overrides {
        let value = serde_json::from_str::<Value>(value_raw)
            .map_err(|e| format!("Invalid project override value for {key}: {e}"))?;
        object.insert(key.clone(), value);
    }

    serde_json::from_value(Value::Object(object))
        .map_err(|e| format!("Failed to deserialize merged settings: {e}"))
}
