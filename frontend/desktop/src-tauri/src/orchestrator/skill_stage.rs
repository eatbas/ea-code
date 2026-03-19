//! Skill selection stage integration for each iteration.

use std::collections::HashSet;
use std::mem;

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{RunEvent, StageEndStatus, *};
use crate::storage::{self, runs, skills};

use crate::orchestrator::prompts::{self, PromptMeta};
use crate::orchestrator::run_setup::compose_agent_context;
use crate::orchestrator::skill_selection::{
    build_selected_skills_section, build_skill_selector_user, parse_skill_selection_output,
};
use crate::orchestrator::stages::execute_agent_stage;
use crate::orchestrator::stages::execute_skipped_stage;
use crate::orchestrator::stages::PauseHandling;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillCatalogEntry {
    id: String,
    name: String,
    description: String,
}

/// Runs the optional skill selection stage and returns prompt text for selected skills.
#[allow(clippy::too_many_arguments)]
pub async fn run_skill_selection_stage(
    app: &AppHandle,
    request: &PipelineRequest,
    settings: &AppSettings,
    cancel_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    pause_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    run_id: &str,
    session_id: &str,
    iter_num: u32,
    meta: &PromptMeta,
    enhanced: &str,
    selected_plan: Option<&str>,
    previous_judge_output: Option<&str>,
    workspace_context: &str,
    run: &mut PipelineRun,
    stages: &mut Vec<StageResult>,
) -> Result<Option<String>, String> {
    let selector = match settings.skill_selector_agent.as_ref() {
        Some(agent) => agent,
        None => {
            stages.push(execute_skipped_stage(
                app,
                run_id,
                iter_num,
                PipelineStage::SkillSelect,
                "No skill selector agent configured.",
            ));
            return Ok(None);
        }
    };

    let skill_files = match skills::list_skills() {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Warning: Failed to list skills: {e}");
            stages.push(execute_skipped_stage(
                app,
                run_id,
                iter_num,
                PipelineStage::SkillSelect,
                "Failed to load skills catalogue.",
            ));
            return Ok(None);
        }
    };

    if skill_files.is_empty() {
        stages.push(execute_skipped_stage(
            app,
            run_id,
            iter_num,
            PipelineStage::SkillSelect,
            "No active skills found.",
        ));
        return Ok(None);
    }

    let catalogue = skill_files
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

    let seq_start = runs::next_sequence(run_id).unwrap_or(1);
    append_stage_start_event(run_id, &PipelineStage::SkillSelect, iter_num, seq_start)?;

    let skill_output_path = runs::artifact_output_path(run_id, iter_num, "skills").ok();
    let skill_output_path_str = skill_output_path.as_ref().map(|p| p.to_string_lossy().to_string());

    let result = execute_agent_stage(
        app,
        run_id,
        iter_num,
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
            context: Some(compose_agent_context(
                prompts::build_skill_selector_system(meta),
                workspace_context,
            )),
            workspace_path: request.workspace_path.clone(),
        },
        settings,
        cancel_flag,
        pause_flag,
        PauseHandling::ResumeWithinStage,
        Some(session_id),
        skill_output_path_str.as_deref(),
    )
    .await;

    let selector_output = result.output.clone();
    let duration_ms = result.duration_ms;

    if result.status == StageStatus::Failed {
        stages.push(result);
        append_stage_end_event(
            run_id,
            &PipelineStage::SkillSelect,
            iter_num,
            seq_start + 1,
            &StageEndStatus::Failed,
            duration_ms,
        )?;
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
    append_stage_end_event(
        run_id,
        &PipelineStage::SkillSelect,
        iter_num,
        seq_start + 1,
        &StageEndStatus::Completed,
        duration_ms,
    )?;

    let known_ids = skill_files
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

    let selected = skill_files
        .into_iter()
        .filter(|skill| decision.selected_skill_ids.contains(&skill.id))
        .collect::<Vec<_>>();

    let section = build_selected_skills_section(&selected);
    if section.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(section))
    }
}

/// Appends a stage_start event to the event log.
fn append_stage_start_event(
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
) -> Result<(), String> {
    let event = RunEvent::StageStart {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
    };
    runs::append_event(run_id, event)
}

/// Appends a stage_end event to the event log.
fn append_stage_end_event(
    run_id: &str,
    stage: &PipelineStage,
    iteration: u32,
    seq: u64,
    status: &StageEndStatus,
    duration_ms: u64,
) -> Result<(), String> {
    let event = RunEvent::StageEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        stage: stage.clone(),
        iteration,
        status: status.clone(),
        duration_ms,
        verdict: None,
    };
    runs::append_event(run_id, event)
}
