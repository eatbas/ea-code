use diesel::prelude::*;

use crate::db::DbPool;
use crate::models::AppSettings;
use crate::schema::settings;

use super::models::{SettingsChangeset, SettingsRow};

/// Loads settings from the database (single row, id = 1).
pub fn get(pool: &DbPool) -> Result<AppSettings, String> {
    let mut conn = super::get_conn(pool)?;

    let row: SettingsRow = settings::table
        .find(1)
        .select(SettingsRow::as_select())
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
        codex_model: normalise_codex_model_csv(&s.codex_model),
        gemini_model: normalise_gemini_model_csv(&s.gemini_model),
        kimi_model: normalise_kimi_model_csv(&s.kimi_model),
        opencode_model: s.opencode_model.clone(),
        prompt_enhancer_model: normalise_stage_model_value(&s.prompt_enhancer_model),
        skill_selector_model: s
            .skill_selector_model
            .as_deref()
            .map(normalise_stage_model_value),
        planner_model: s.planner_model.as_deref().map(normalise_stage_model_value),
        plan_auditor_model: s
            .plan_auditor_model
            .as_deref()
            .map(normalise_stage_model_value),
        generator_model: normalise_stage_model_value(&s.coder_model),
        reviewer_model: normalise_stage_model_value(&s.code_reviewer_model),
        fixer_model: normalise_stage_model_value(&s.code_fixer_model),
        final_judge_model: normalise_stage_model_value(&s.final_judge_model),
        executive_summary_model: normalise_stage_model_value(&s.executive_summary_model),
        require_plan_approval: s.require_plan_approval,
        plan_auto_approve_timeout_sec: s.plan_auto_approve_timeout_sec as i32,
        max_plan_revisions: s.max_plan_revisions as i32,
        token_optimized_prompts: s.token_optimized_prompts,
        agent_retry_count: s.agent_retry_count as i32,
        agent_timeout_ms: s.agent_timeout_ms as i32,
        agent_max_turns: normalise_agent_max_turns(s.agent_max_turns) as i32,
        retention_days: s.retention_days as i32,
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
        codex_model: normalise_codex_model_csv(&row.codex_model),
        gemini_model: normalise_gemini_model_csv(&row.gemini_model),
        kimi_model: normalise_kimi_model_csv(&row.kimi_model),
        opencode_model: row.opencode_model.clone(),
        prompt_enhancer_model: normalise_stage_model_value(&row.prompt_enhancer_model),
        skill_selector_model: row
            .skill_selector_model
            .as_deref()
            .map(normalise_stage_model_value),
        planner_model: row.planner_model.as_deref().map(normalise_stage_model_value),
        plan_auditor_model: row
            .plan_auditor_model
            .as_deref()
            .map(normalise_stage_model_value),
        coder_model: normalise_stage_model_value(&row.generator_model),
        code_reviewer_model: normalise_stage_model_value(&row.reviewer_model),
        code_fixer_model: normalise_stage_model_value(&row.fixer_model),
        final_judge_model: normalise_stage_model_value(&row.final_judge_model),
        executive_summary_model: normalise_stage_model_value(&row.executive_summary_model),
        require_plan_approval: row.require_plan_approval,
        plan_auto_approve_timeout_sec: row.plan_auto_approve_timeout_sec as u32,
        max_plan_revisions: row.max_plan_revisions as u32,
        token_optimized_prompts: row.token_optimized_prompts,
        agent_retry_count: row.agent_retry_count as u32,
        agent_timeout_ms: row.agent_timeout_ms as u64,
        agent_max_turns: normalise_agent_max_turns_from_db(row.agent_max_turns),
        retention_days: row.retention_days as u32,
    }
}

fn normalise_agent_max_turns(value: u32) -> u32 {
    if value == 0 {
        return 25;
    }
    value.min(100)
}

fn normalise_agent_max_turns_from_db(value: i32) -> u32 {
    if value <= 0 {
        return 25;
    }
    normalise_agent_max_turns(value as u32)
}

fn normalise_kimi_model_csv(csv: &str) -> String {
    csv.split(',')
        .map(normalise_kimi_model_value)
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(",")
}

fn normalise_codex_model_csv(csv: &str) -> String {
    csv.split(',')
        .map(normalise_codex_model_value)
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(",")
}

fn normalise_gemini_model_csv(csv: &str) -> String {
    csv.split(',')
        .map(normalise_gemini_model_value)
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(",")
}

fn normalise_codex_model_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed == "codex-5.3" {
        return "gpt-5.3-codex".to_string();
    }
    trimmed.to_string()
}

fn normalise_gemini_model_value(value: &str) -> String {
    let trimmed = value.trim();
    match trimmed {
        // Backward compatibility: old preview names no longer accepted by Gemini CLI.
        "gemini-3.0-flash-preview" => "gemini-3-flash-preview".to_string(),
        "gemini-3.0-flash" => "gemini-3-flash-preview".to_string(),
        _ => trimmed.to_string(),
    }
}

fn normalise_stage_model_value(value: &str) -> String {
    normalise_gemini_model_value(&normalise_codex_model_value(value))
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
