use crate::conversations::persistence::{
    load_pipeline_state, save_pipeline_state, update_pipeline_stage,
};
use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};

use super::helpers::TestWorkspace;

fn stage(stage_index: usize, stage_name: &str) -> PipelineStageRecord {
    PipelineStageRecord {
        stage_index,
        stage_name: stage_name.to_string(),
        agent_label: "test / model".to_string(),
        status: ConversationStatus::Running,
        text: String::new(),
        started_at: Some("2026-04-12T00:00:00Z".to_string()),
        finished_at: None,
        score_id: None,
        provider_session_ref: None,
    }
}

#[test]
fn update_pipeline_stage_matches_stage_index_not_vector_position() {
    let workspace = TestWorkspace::new();
    let workspace_path = workspace
        .path()
        .to_str()
        .expect("workspace path should be utf-8");
    let conversation_id = "conv-1";
    let conversation_dir = workspace
        .path()
        .join(".maestro")
        .join("conversations")
        .join(conversation_id);
    std::fs::create_dir_all(&conversation_dir).expect("conversation directory should exist");

    let initial = PipelineState {
        user_prompt: "prompt".to_string(),
        pipeline_mode: "code".to_string(),
        stages: vec![
            stage(0, "Prompt Enhancer"),
            stage(1, "Planner 1"),
            stage(3, "Plan Merge"),
        ],
        review_cycle: 1,
        enhanced_prompt: None,
    };
    save_pipeline_state(workspace_path, conversation_id, &initial)
        .expect("pipeline state should save");

    let mut updated = stage(3, "Plan Merge");
    updated.status = ConversationStatus::Completed;
    updated.score_id = Some("score-merge".to_string());
    updated.provider_session_ref = Some("session-merge".to_string());
    updated.finished_at = Some("2026-04-12T00:01:00Z".to_string());
    update_pipeline_stage(workspace_path, conversation_id, &updated)
        .expect("pipeline stage should update");

    let loaded = load_pipeline_state(workspace_path, conversation_id)
        .expect("pipeline state should load")
        .expect("pipeline state should exist");

    let planner = loaded
        .stages
        .iter()
        .find(|stage| stage.stage_index == 1)
        .expect("planner should exist");
    assert_eq!(planner.stage_name, "Planner 1");
    assert_eq!(planner.score_id, None);

    let merge = loaded
        .stages
        .iter()
        .find(|stage| stage.stage_index == 3)
        .expect("plan merge should exist");
    assert_eq!(merge.stage_name, "Plan Merge");
    assert_eq!(merge.score_id.as_deref(), Some("score-merge"));
    assert_eq!(merge.provider_session_ref.as_deref(), Some("session-merge"));
}

#[test]
fn update_pipeline_stage_appends_missing_stage_record() {
    let workspace = TestWorkspace::new();
    let workspace_path = workspace
        .path()
        .to_str()
        .expect("workspace path should be utf-8");
    let conversation_id = "conv-2";
    let conversation_dir = workspace
        .path()
        .join(".maestro")
        .join("conversations")
        .join(conversation_id);
    std::fs::create_dir_all(&conversation_dir).expect("conversation directory should exist");

    let initial = PipelineState {
        user_prompt: "prompt".to_string(),
        pipeline_mode: "code".to_string(),
        stages: vec![stage(0, "Planner 1")],
        review_cycle: 1,
        enhanced_prompt: None,
    };
    save_pipeline_state(workspace_path, conversation_id, &initial)
        .expect("pipeline state should save");

    let mut missing = stage(2, "Plan Merge");
    missing.score_id = Some("score-merge".to_string());
    update_pipeline_stage(workspace_path, conversation_id, &missing)
        .expect("missing stage should be appended");

    let loaded = load_pipeline_state(workspace_path, conversation_id)
        .expect("pipeline state should load")
        .expect("pipeline state should exist");

    assert!(loaded.stages.iter().any(|stage| {
        stage.stage_index == 2
            && stage.stage_name == "Plan Merge"
            && stage.score_id.as_deref() == Some("score-merge")
    }));
}
