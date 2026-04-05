//! Pipeline configuration loading and runtime state allocation.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::models::{CodePipelineSettings, PipelineAgent};

use super::super::super::persistence;

/// Precomputed stage index layout for the full pipeline.
#[allow(dead_code)]
pub(in crate::conversations::commands) struct StageIndices {
    pub orchestrator: Option<usize>,
    pub planner_count: usize,
    pub reviewer_count: usize,
    pub plan_merge: usize,
    pub coder: usize,
    pub reviewer_start: usize,
    pub review_merge: usize,
    pub code_fixer: usize,
    pub total: usize,
}

impl StageIndices {
    pub fn new(planner_count: usize, reviewer_count: usize, has_orchestrator: bool) -> Self {
        if has_orchestrator {
            // Orchestrator occupies index 0, everything else shifts by +1.
            Self {
                orchestrator: Some(0),
                planner_count,
                reviewer_count,
                plan_merge: planner_count + 1,
                coder: planner_count + 2,
                reviewer_start: planner_count + 3,
                review_merge: planner_count + 3 + reviewer_count,
                code_fixer: planner_count + 4 + reviewer_count,
                total: planner_count + 5 + reviewer_count,
            }
        } else {
            // No orchestrator, indices remain unchanged for backwards compatibility.
            Self {
                orchestrator: None,
                planner_count,
                reviewer_count,
                plan_merge: planner_count,
                coder: planner_count + 1,
                reviewer_start: planner_count + 2,
                review_merge: planner_count + 2 + reviewer_count,
                code_fixer: planner_count + 3 + reviewer_count,
                total: planner_count + 4 + reviewer_count,
            }
        }
    }
}

/// Pipeline configuration loaded from settings before runtime state is allocated.
pub(in crate::conversations::commands) struct PipelineConfig {
    pub orchestrator_agent: Option<PipelineAgent>,
    pub planners: Vec<PipelineAgent>,
    pub planner_count: usize,
    pub merge_agent: PipelineAgent,
    pub coder: PipelineAgent,
    pub reviewers: Vec<PipelineAgent>,
    pub reviewer_count: usize,
    pub indices: StageIndices,
}

/// Pre-allocated runtime state shared by all pipeline handler spawn blocks.
#[allow(dead_code)]
pub(in crate::conversations::commands) struct PipelineSetup {
    pub orchestrator_agent: Option<PipelineAgent>,
    pub abort: Arc<AtomicBool>,
    pub score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    pub stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
    pub planners: Vec<PipelineAgent>,
    pub planner_count: usize,
    pub merge_agent: PipelineAgent,
    pub coder: PipelineAgent,
    pub reviewers: Vec<PipelineAgent>,
    pub reviewer_count: usize,
    pub indices: StageIndices,
}

/// Load pipeline settings without allocating runtime state.
pub(in crate::conversations::commands) fn load_pipeline_config() -> Result<PipelineConfig, String> {
    let settings = crate::storage::settings::read_settings()?;
    let config: CodePipelineSettings = settings
        .code_pipeline
        .ok_or("Code pipeline is not configured. Set it up in Agents settings.")?;
    let CodePipelineSettings {
        planners, coder, ..
    } = config;

    let planner_count = planners.len();
    if planner_count == 0 {
        return Err("No planners configured".to_string());
    }
    let merge_agent = planners[0].clone();
    let reviewers = planners.clone();
    let reviewer_count = planner_count;

    // Read orchestrator agent from settings.
    let orchestrator_agent = settings.orchestrator.as_ref().map(|o| o.agent.clone());
    let has_orchestrator = orchestrator_agent.is_some();

    let indices = StageIndices::new(planner_count, reviewer_count, has_orchestrator);

    Ok(PipelineConfig {
        orchestrator_agent,
        planners,
        planner_count,
        merge_agent,
        coder,
        reviewers,
        reviewer_count,
        indices,
    })
}

/// Allocate abort/slot/buffer registries for a specific conversation.
pub(in crate::conversations::commands) fn prepare_pipeline_with_config(
    workspace_path: &str,
    conversation_id: &str,
    config: PipelineConfig,
) -> Result<PipelineSetup, String> {
    let PipelineConfig {
        orchestrator_agent,
        planners,
        planner_count,
        merge_agent,
        coder,
        reviewers,
        reviewer_count,
        indices,
    } = config;

    let abort = persistence::register_abort_flag(workspace_path, conversation_id)?;
    let score_id_slots =
        persistence::register_pipeline_score_slots(workspace_path, conversation_id, indices.total)?;
    let stage_buffers = persistence::register_pipeline_stage_buffers(
        workspace_path,
        conversation_id,
        indices.total,
    )?;

    Ok(PipelineSetup {
        orchestrator_agent,
        abort,
        score_id_slots,
        stage_buffers,
        planners,
        planner_count,
        merge_agent,
        coder,
        reviewers,
        reviewer_count,
        indices,
    })
}

/// Load pipeline settings and allocate abort/slot/buffer registries.
/// Shared by resume_pipeline and send_plan_edit_feedback.
pub(in crate::conversations::commands) fn prepare_pipeline(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<PipelineSetup, String> {
    prepare_pipeline_with_config(workspace_path, conversation_id, load_pipeline_config()?)
}
