//! Skill selection stage integration for each iteration.

use std::collections::HashSet;
use std::mem;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::models::*;

use super::helpers::*;
use super::prompts::{self, PromptMeta};
use super::skill_selection::{
    build_selected_skills_section, build_skill_selector_user, parse_skill_selection_output,
};
use super::stages::*;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillCatalogEntry {
    id: String,
    name: String,
    description: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillSelectionArtifact<'a> {
    selected_skill_ids: &'a [String],
    reason: &'a str,
}

/// Runs the optional skill selection stage and returns prompt text for selected skills.
#[allow(clippy::too_many_arguments)]
pub async fn run_skill_selection_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    db_pool: &DbPool,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    iteration_db_id: i32,
    meta: &PromptMeta,
    enhanced: &str,
    selected_plan: Option<&str>,
    previous_judge_output: Option<&str>,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
) -> Result<Option<String>, String> {
    if !settings.skill_selection_mode.eq_ignore_ascii_case("auto") {
        stages.push(execute_skipped_stage(
            app,
            run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::SkillSelect,
            "Skill selection mode is disabled.",
            db_pool,
        ));
        return Ok(None);
    }

    let selector = match settings.skill_selector_agent.as_ref() {
        Some(agent) => agent,
        None => {
            stages.push(execute_skipped_stage(
                app,
                run_id,
                iter_num,
                iteration_db_id,
                PipelineStage::SkillSelect,
                "No skill selector agent configured.",
                db_pool,
            ));
            return Ok(None);
        }
    };

    let skills = db::skills::list_active(db_pool)?
        .into_iter()
        .map(|row| Skill {
            id: row.id,
            name: row.name,
            description: row.description,
            instructions: row.instructions,
            tags: row.tags,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect::<Vec<_>>();

    if skills.is_empty() {
        stages.push(execute_skipped_stage(
            app,
            run_id,
            iter_num,
            iteration_db_id,
            PipelineStage::SkillSelect,
            "No active skills found.",
            db_pool,
        ));
        return Ok(None);
    }

    let catalogue = skills
        .iter()
        .map(|skill| SkillCatalogEntry {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
        })
        .collect::<Vec<_>>();
    let skill_catalog_json = serde_json::to_string_pretty(&catalogue)
        .map_err(|e| format!("Failed to serialise skill catalogue: {e}"))?;

    run.current_stage = Some(PipelineStage::SkillSelect);
    let result = execute_agent_stage(
        app,
        run_id,
        iter_num,
        iteration_db_id,
        PipelineStage::SkillSelect,
        selector,
        &AgentInput {
            prompt: build_skill_selector_user(
                &request.prompt,
                enhanced,
                selected_plan,
                previous_judge_output,
                &skill_catalog_json,
            ),
            context: Some(prompts::build_skill_selector_system(meta)),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        Some(session_id),
        db_pool,
    )
    .await;

    let selector_output = result.output.clone();
    if result.status == StageStatus::Failed {
        stages.push(result);
        run.iterations.push(Iteration {
            number: iter_num,
            stages: mem::take(stages),
            verdict: None,
            judge_reasoning: None,
        });
        run.status = PipelineStatus::Failed;
        run.error = Some("Skill selector stage failed".to_string());
        return Ok(None);
    }
    stages.push(result);

    let known_ids = skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<HashSet<_>>();

    let decision = match parse_skill_selection_output(&selector_output, &known_ids, 3) {
        Ok(parsed) => parsed,
        Err(error) => super::skill_selection::SkillSelectionDecision {
            selected_skill_ids: Vec::new(),
            reason: format!("Selector parse fallback: {error}"),
        },
    };

    let selected = skills
        .into_iter()
        .filter(|skill| decision.selected_skill_ids.contains(&skill.id))
        .collect::<Vec<_>>();

    let artifact = serde_json::to_string_pretty(&SkillSelectionArtifact {
        selected_skill_ids: &decision.selected_skill_ids,
        reason: &decision.reason,
    })
    .unwrap_or_else(|_| "{\"selectedSkillIds\":[],\"reason\":\"serialisation error\"}".to_string());
    emit_artifact(app, run_id, "selected_skills", &artifact, iter_num, db_pool);

    let section = build_selected_skills_section(&selected);
    if section.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(section))
    }
}
