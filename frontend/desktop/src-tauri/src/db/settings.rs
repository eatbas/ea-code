use diesel::prelude::*;

use crate::db::DbPool;
use crate::models::AppSettings;
use crate::schema::settings;

use super::models::{SettingsChangeset, SettingsRow};

const FIXED_AGENT_MAX_TURNS: u32 = 25;

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
        prompt_enhancer_agent: backend_to_db_str_or_empty(s.prompt_enhancer_agent.as_ref()),
        skill_selector_agent: backend_to_opt(s.skill_selector_agent.as_ref()),
        planner_agent: backend_to_opt(s.planner_agent.as_ref()),
        plan_auditor_agent: backend_to_opt(s.plan_auditor_agent.as_ref()),
        generator_agent: backend_to_db_str_or_empty(s.coder_agent.as_ref()),
        reviewer_agent: backend_to_db_str_or_empty(s.code_reviewer_agent.as_ref()),
        fixer_agent: backend_to_db_str_or_empty(s.code_fixer_agent.as_ref()),
        final_judge_agent: backend_to_db_str_or_empty(s.final_judge_agent.as_ref()),
        executive_summary_agent: backend_to_db_str_or_empty(s.executive_summary_agent.as_ref()),
        max_iterations: s.max_iterations as i32,
        require_git: s.require_git,
        updated_at: super::now_rfc3339(),
        claude_model: s.claude_model.clone(),
        codex_model: s.codex_model.clone(),
        gemini_model: s.gemini_model.clone(),
        kimi_model: normalise_kimi_model_csv(&s.kimi_model),
        opencode_model: s.opencode_model.clone(),
        copilot_model: s.copilot_model.clone(),
        prompt_enhancer_model: s.prompt_enhancer_model.clone(),
        skill_selector_model: s.skill_selector_model.clone(),
        planner_model: s.planner_model.clone(),
        plan_auditor_model: s.plan_auditor_model.clone(),
        generator_model: s.coder_model.clone(),
        reviewer_model: s.code_reviewer_model.clone(),
        fixer_model: s.code_fixer_model.clone(),
        final_judge_model: s.final_judge_model.clone(),
        executive_summary_model: s.executive_summary_model.clone(),
        require_plan_approval: s.require_plan_approval,
        plan_auto_approve_timeout_sec: s.plan_auto_approve_timeout_sec as i32,
        max_plan_revisions: s.max_plan_revisions as i32,
        token_optimized_prompts: s.token_optimized_prompts,
        agent_retry_count: s.agent_retry_count as i32,
        agent_timeout_ms: s.agent_timeout_ms as i32,
        agent_max_turns: FIXED_AGENT_MAX_TURNS as i32,
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

    fn parse_backend(s: &str) -> Option<AgentBackend> {
        match s {
            "claude" => Some(AgentBackend::Claude),
            "codex" => Some(AgentBackend::Codex),
            "gemini" => Some(AgentBackend::Gemini),
            "kimi" => Some(AgentBackend::Kimi),
            // Legacy compatibility: Copilot assignments are migrated to Codex.
            "copilot" => Some(AgentBackend::Codex),
            "opencode" => Some(AgentBackend::OpenCode),
            "" => None,
            _ => {
                eprintln!("Unknown backend in settings row: {s}; leaving unset");
                None
            }
        }
    }

    fn parse_optional_backend(s: Option<&str>) -> Option<AgentBackend> {
        s.and_then(parse_backend)
    }

    fn parse_required_backend(raw: &str) -> Option<AgentBackend> {
        parse_backend(raw.trim())
    }

    AppSettings {
        claude_path: row.claude_path.clone(),
        codex_path: row.codex_path.clone(),
        gemini_path: row.gemini_path.clone(),
        kimi_path: row.kimi_path.clone(),
        opencode_path: row.opencode_path.clone(),
        copilot_path: row.copilot_path.clone(),
        prompt_enhancer_agent: parse_required_backend(&row.prompt_enhancer_agent),
        skill_selector_agent: parse_optional_backend(row.skill_selector_agent.as_deref()),
        planner_agent: parse_optional_backend(row.planner_agent.as_deref()),
        plan_auditor_agent: parse_optional_backend(row.plan_auditor_agent.as_deref()),
        coder_agent: parse_required_backend(&row.generator_agent),
        code_reviewer_agent: parse_required_backend(&row.reviewer_agent),
        code_fixer_agent: parse_required_backend(&row.fixer_agent),
        final_judge_agent: parse_required_backend(&row.final_judge_agent),
        executive_summary_agent: parse_required_backend(&row.executive_summary_agent),
        max_iterations: row.max_iterations as u32,
        require_git: row.require_git,
        claude_model: row.claude_model.clone(),
        codex_model: row.codex_model.clone(),
        gemini_model: row.gemini_model.clone(),
        kimi_model: normalise_kimi_model_csv(&row.kimi_model),
        opencode_model: row.opencode_model.clone(),
        copilot_model: row.copilot_model.clone(),
        prompt_enhancer_model: row.prompt_enhancer_model.clone(),
        skill_selector_model: row.skill_selector_model.clone(),
        planner_model: row.planner_model.clone(),
        plan_auditor_model: row.plan_auditor_model.clone(),
        coder_model: row.generator_model.clone(),
        code_reviewer_model: row.reviewer_model.clone(),
        code_fixer_model: row.fixer_model.clone(),
        final_judge_model: row.final_judge_model.clone(),
        executive_summary_model: row.executive_summary_model.clone(),
        require_plan_approval: row.require_plan_approval,
        plan_auto_approve_timeout_sec: row.plan_auto_approve_timeout_sec as u32,
        max_plan_revisions: row.max_plan_revisions as u32,
        token_optimized_prompts: row.token_optimized_prompts,
        agent_retry_count: row.agent_retry_count as u32,
        agent_timeout_ms: row.agent_timeout_ms as u64,
        agent_max_turns: FIXED_AGENT_MAX_TURNS,
    }
}

fn normalise_kimi_model_csv(csv: &str) -> String {
    csv.split(',')
        .map(normalise_kimi_model_value)
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(",")
}

fn normalise_kimi_model_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Backward compatibility: expand legacy short aliases.
    if trimmed == "kimi-for-coding" || trimmed == "kimi-code" {
        return "kimi-code/kimi-for-coding".to_string();
    }
    trimmed.to_string()
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

fn backend_to_db_str_or_empty(b: Option<&crate::models::AgentBackend>) -> String {
    b.map(backend_to_str).unwrap_or_default()
}
