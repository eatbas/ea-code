//! High-level chain runners: merge, review-merge, and full coding phase.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::{ConversationStatus, PipelineAgent, PipelineStageRecord};

use super::super::super::persistence;
use super::super::super::pipeline;
use super::lifecycle::{ensure_merge_stage_record, ensure_stage_record};
use super::setup::{PipelineSetup, StageIndices};

fn planner_stage_range(indices: &StageIndices) -> std::ops::Range<usize> {
    let start = indices.orchestrator.map(|_| 1).unwrap_or(0);
    start..start + indices.planner_count
}

fn collect_planner_stages(
    stages: Vec<PipelineStageRecord>,
    indices: &StageIndices,
) -> Vec<PipelineStageRecord> {
    let planner_range = planner_stage_range(indices);
    let mut planner_stages: Vec<_> = stages
        .into_iter()
        .filter(|stage| planner_range.contains(&stage.stage_index))
        .collect();
    planner_stages.sort_by_key(|stage| stage.stage_index);
    planner_stages
}

/// Run the merge chain: load session ref, ensure record, call run_plan_merge.
/// Returns None if the merge was skipped (no session ref available).
pub(in crate::conversations::commands) async fn run_merge_chain(
    app: AppHandle,
    conv_id: String,
    ws: String,
    abort: Arc<AtomicBool>,
    merge_agent: PipelineAgent,
    plan_merge_index: usize,
    planner_count: usize,
    score_id_slots: &[Arc<std::sync::Mutex<Option<String>>>],
    stage_buffers: &[Arc<std::sync::Mutex<String>>],
) -> Option<Result<PipelineStageRecord, (PipelineStageRecord, String)>> {
    let loaded = persistence::load_pipeline_state(&ws, &conv_id)
        .ok()
        .flatten();
    // Find the first planner stage for session ref (not orchestrator).
    let session_ref = loaded.as_ref().and_then(|s| {
        s.stages
            .iter()
            .find(|st| st.stage_name.starts_with("Planner"))
            .and_then(|st| st.provider_session_ref.clone())
    });

    let ref_val = match session_ref {
        Some(v) => v,
        None => {
            eprintln!("[pipeline] No provider_session_ref from first planner; skipping merge");
            return None;
        }
    };

    let merge_label = format!("{} / {}", merge_agent.provider, merge_agent.model);
    ensure_merge_stage_record(&ws, &conv_id, plan_merge_index, &merge_label);

    let merge_slot = score_id_slots
        .get(plan_merge_index)
        .cloned()
        .unwrap_or_default();
    let merge_buf = stage_buffers
        .get(plan_merge_index)
        .cloned()
        .unwrap_or_default();

    Some(
        pipeline::run_plan_merge(
            app,
            conv_id,
            ws,
            abort,
            merge_slot,
            merge_buf,
            plan_merge_index,
            planner_count,
            ref_val,
            merge_agent,
        )
        .await,
    )
}

/// Run the review-merge chain: load first reviewer's session ref, ensure
/// stage record, call run_review_merge.
pub(in crate::conversations::commands) async fn run_review_merge_chain(
    app: AppHandle,
    conv_id: String,
    ws: String,
    abort: Arc<AtomicBool>,
    review_merge_agent: PipelineAgent,
    indices: &StageIndices,
    score_id_slots: &[Arc<std::sync::Mutex<Option<String>>>],
    stage_buffers: &[Arc<std::sync::Mutex<String>>],
) -> Option<Result<PipelineStageRecord, (PipelineStageRecord, String)>> {
    let loaded = persistence::load_pipeline_state(&ws, &conv_id)
        .ok()
        .flatten();
    let session_ref = loaded.as_ref().and_then(|s| {
        s.stages
            .iter()
            .find(|st| st.stage_index == indices.reviewer_start)
            .and_then(|st| st.provider_session_ref.clone())
    });

    let ref_val = match session_ref {
        Some(v) => v,
        None => {
            eprintln!(
                "[pipeline] No provider_session_ref from first reviewer; skipping review merge"
            );
            return None;
        }
    };

    let label = format!(
        "{} / {}",
        review_merge_agent.provider, review_merge_agent.model
    );
    ensure_stage_record(&ws, &conv_id, indices.review_merge, "Review Merge", &label);

    let slot = score_id_slots
        .get(indices.review_merge)
        .cloned()
        .unwrap_or_default();
    let buf = stage_buffers
        .get(indices.review_merge)
        .cloned()
        .unwrap_or_default();

    Some(
        pipeline::run_review_merge(
            app,
            conv_id,
            ws,
            abort,
            slot,
            buf,
            indices.review_merge,
            indices.reviewer_count,
            ref_val,
            review_merge_agent,
            None,
            None,
            None,
        )
        .await,
    )
}

/// Run the full coding phase: Coder -> Reviewers -> Review Merge -> Code Fixer.
/// Returns the final status and optional error.
pub(in crate::conversations::commands) async fn run_coding_phase(
    app: AppHandle,
    conv_id: String,
    ws: String,
    setup: &PipelineSetup,
    previous_stages: Option<Vec<PipelineStageRecord>>,
) -> (ConversationStatus, Option<String>) {
    let indices = &setup.indices;

    // Check if the Coder stage is already complete (e.g. resuming from Reviewers).
    let coder_already_done = previous_stages
        .as_ref()
        .and_then(|stages| stages.iter().find(|s| s.stage_name == "Coder"))
        .map(|s| s.status == ConversationStatus::Completed)
        .unwrap_or(false);

    let coder_record = if coder_already_done {
        // Re-emit the completed Coder stage for the frontend and use saved record.
        let loaded = persistence::load_pipeline_state(&ws, &conv_id)
            .ok()
            .flatten();
        loaded
            .and_then(|s| s.stages.into_iter().find(|st| st.stage_name == "Coder"))
            .unwrap_or_else(|| PipelineStageRecord {
                stage_index: indices.coder,
                stage_name: "Coder".to_string(),
                agent_label: format!("{} / {}", setup.coder.provider, setup.coder.model),
                status: ConversationStatus::Completed,
                text: String::new(),
                started_at: None,
                finished_at: None,
                score_id: None,
                provider_session_ref: None,
                user_prompt: None,
            })
    } else {
        // --- Coder ---
        let coder_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
        ensure_stage_record(&ws, &conv_id, indices.coder, "Coder", &coder_label);

        let coder_slot = setup
            .score_id_slots
            .get(indices.coder)
            .cloned()
            .unwrap_or_default();
        let coder_buf = setup
            .stage_buffers
            .get(indices.coder)
            .cloned()
            .unwrap_or_default();

        let coder_result = pipeline::run_coder(
            app.clone(),
            conv_id.clone(),
            ws.clone(),
            setup.abort.clone(),
            coder_slot,
            coder_buf,
            indices.coder,
            setup.coder.clone(),
        )
        .await;

        if setup.abort.load(Ordering::Acquire) {
            return (ConversationStatus::Stopped, None);
        }

        match coder_result {
            Ok(record) => record,
            Err((_, e)) => return (ConversationStatus::Failed, Some(e)),
        }
    };

    // Collect planner stages for reviewer session resumption.
    let planner_stages: Vec<PipelineStageRecord> = persistence::load_pipeline_state(&ws, &conv_id)
        .ok()
        .flatten()
        .map(|state| collect_planner_stages(state.stages, indices))
        .unwrap_or_default();

    // --- Reviewers ---
    let reviewer_slots: Vec<_> = (0..indices.reviewer_count)
        .map(|i| {
            setup
                .score_id_slots
                .get(indices.reviewer_start + i)
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    let reviewer_bufs: Vec<_> = (0..indices.reviewer_count)
        .map(|i| {
            setup
                .stage_buffers
                .get(indices.reviewer_start + i)
                .cloned()
                .unwrap_or_default()
        })
        .collect();

    // Ensure reviewer stage records exist before spawning.
    for (i, reviewer) in setup.reviewers.iter().enumerate() {
        let label = format!("{} / {}", reviewer.provider, reviewer.model);
        ensure_stage_record(
            &ws,
            &conv_id,
            indices.reviewer_start + i,
            &format!("Reviewer {}", i + 1),
            &label,
        );
    }

    let reviewer_result = pipeline::run_pipeline_reviewers(
        app.clone(),
        conv_id.clone(),
        ws.clone(),
        setup.reviewers.clone(),
        setup.abort.clone(),
        reviewer_slots,
        previous_stages,
        reviewer_bufs,
        &planner_stages,
        indices.reviewer_start,
        None,
        None,
    )
    .await;

    if setup.abort.load(Ordering::Acquire) {
        return (ConversationStatus::Stopped, None);
    }

    if let Err(e) = reviewer_result {
        return (ConversationStatus::Failed, Some(e));
    }

    // --- Review Merge ---
    let Some(review_merge_agent) = setup.reviewers.first().cloned() else {
        return (
            ConversationStatus::Failed,
            Some("No reviewer available for Review Merge".to_string()),
        );
    };

    let review_merge_result = run_review_merge_chain(
        app.clone(),
        conv_id.clone(),
        ws.clone(),
        setup.abort.clone(),
        review_merge_agent,
        indices,
        &setup.score_id_slots,
        &setup.stage_buffers,
    )
    .await;

    if setup.abort.load(Ordering::Acquire) {
        return (ConversationStatus::Stopped, None);
    }

    match &review_merge_result {
        Some(Err((_, e))) => return (ConversationStatus::Failed, Some(e.clone())),
        None => {
            return (
                ConversationStatus::Failed,
                Some("Review merge was skipped — no reviewer session ref".to_string()),
            );
        }
        Some(Ok(_)) => {}
    }

    // --- Code Fixer ---
    let coder_session_ref = coder_record
        .provider_session_ref
        .clone()
        .unwrap_or_default();

    if coder_session_ref.is_empty() {
        return (
            ConversationStatus::Failed,
            Some("No coder session ref available for Code Fixer".to_string()),
        );
    }

    let fixer_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
    ensure_stage_record(
        &ws,
        &conv_id,
        indices.code_fixer,
        "Code Fixer",
        &fixer_label,
    );

    let fixer_slot = setup
        .score_id_slots
        .get(indices.code_fixer)
        .cloned()
        .unwrap_or_default();
    let fixer_buf = setup
        .stage_buffers
        .get(indices.code_fixer)
        .cloned()
        .unwrap_or_default();

    let fixer_result = pipeline::run_code_fixer(
        app,
        conv_id,
        ws,
        setup.abort.clone(),
        fixer_slot,
        fixer_buf,
        indices.code_fixer,
        coder_session_ref,
        setup.coder.clone(),
        None,
        None,
        None,
    )
    .await;

    if setup.abort.load(Ordering::Acquire) {
        return (ConversationStatus::Stopped, None);
    }

    match fixer_result {
        Ok(_) => (ConversationStatus::Completed, None),
        Err((_, e)) => (ConversationStatus::Failed, Some(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_planner_stages, planner_stage_range};
    use crate::models::{ConversationStatus, PipelineStageRecord};
    use crate::storage::now_rfc3339;

    use super::super::setup::StageIndices;

    fn stage(
        stage_index: usize,
        stage_name: &str,
        provider_session_ref: Option<&str>,
    ) -> PipelineStageRecord {
        PipelineStageRecord {
            stage_index,
            stage_name: stage_name.to_string(),
            agent_label: "test / model".to_string(),
            status: ConversationStatus::Completed,
            text: String::new(),
            started_at: Some(now_rfc3339()),
            finished_at: Some(now_rfc3339()),
            score_id: Some(format!("score-{stage_index}")),
            provider_session_ref: provider_session_ref.map(str::to_string),
            user_prompt: None,
        }
    }

    #[test]
    fn planner_stage_range_skips_orchestrator() {
        let indices = StageIndices::new(2, 2, true);
        assert_eq!(planner_stage_range(&indices), 1..3);
    }

    #[test]
    fn collect_planner_stages_uses_stage_indices() {
        let indices = StageIndices::new(2, 2, true);
        let planner_stages = collect_planner_stages(
            vec![
                stage(0, "Prompt Enhancer", None),
                stage(1, "Planner 1", Some("planner-1")),
                stage(2, "Planner 2", Some("planner-2")),
                stage(3, "Plan Merge", Some("merge")),
            ],
            &indices,
        );

        assert_eq!(planner_stages.len(), 2);
        assert_eq!(planner_stages[0].stage_name, "Planner 1");
        assert_eq!(
            planner_stages[0].provider_session_ref.as_deref(),
            Some("planner-1")
        );
        assert_eq!(planner_stages[1].stage_name, "Planner 2");
        assert_eq!(
            planner_stages[1].provider_session_ref.as_deref(),
            Some("planner-2")
        );
    }
}
