//! The resume_pipeline command — determines where the pipeline stopped and
//! restarts from that point.

use std::sync::atomic::Ordering;

use tauri::AppHandle;

use crate::models::{ConversationDetail, ConversationStatus};

use super::super::super::persistence;
use super::super::super::pipeline;
use super::super::pipeline_orchestration::{
    begin_pipeline_task, determine_final_status, emit_final_status, ensure_stage_record,
    pipeline_cleanup, prepare_pipeline, re_emit_completed_stages, run_coding_phase, run_merge_chain,
    run_review_merge_chain,
};

#[tauri::command]
pub async fn resume_pipeline(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
) -> Result<ConversationDetail, String> {
    let state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    let setup = prepare_pipeline(&workspace_path, &conversation_id)?;

    let user_prompt = state.user_prompt;
    let previous_stages = state.stages;
    let indices = &setup.indices;

    // Determine which stages are complete.
    let stage_done = |name: &str| -> bool {
        previous_stages
            .iter()
            .find(|s| s.stage_name == name)
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false)
    };
    let all_planners_done = previous_stages
        .iter()
        .take(setup.planner_count)
        .all(|s| s.status == ConversationStatus::Completed);
    let merge_done = stage_done("Plan Merge");
    let coder_done = stage_done("Coder");
    let all_reviewers_done = (0..indices.reviewer_count).all(|i| {
        previous_stages
            .iter()
            .find(|s| s.stage_index == indices.reviewer_start + i)
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false)
    });
    let review_merge_done = stage_done("Review Merge");
    let code_fixer_done = stage_done("Code Fixer");

    // Determine resume point.
    #[derive(Debug)]
    enum ResumePoint {
        Planners,
        PlanMerge,
        CodingPhase,       // Coder not done — run full coding phase
        Reviewers,         // Coder done, reviewers not all done
        ReviewMerge,       // Reviewers done, review merge not done
        CodeFixer,         // Review merge done, code fixer not done
        AlreadyComplete,
    }

    let resume_point = if code_fixer_done {
        ResumePoint::AlreadyComplete
    } else if review_merge_done {
        ResumePoint::CodeFixer
    } else if all_reviewers_done && coder_done {
        ResumePoint::ReviewMerge
    } else if coder_done {
        ResumePoint::Reviewers
    } else if all_planners_done && merge_done {
        ResumePoint::CodingPhase
    } else if all_planners_done {
        ResumePoint::PlanMerge
    } else {
        ResumePoint::Planners
    };

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let prev = previous_stages.clone();

    let planner_count = setup.planner_count;
    let indices_coder = indices.coder;

    tokio::spawn(async move {
        let Some(_guard) = begin_pipeline_task(&app_handle, &ws, &conv_id) else {
            return;
        };

        match resume_point {
            ResumePoint::AlreadyComplete => {
                emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Completed, None);
            }
            ResumePoint::Planners => {
                let planner_result = pipeline::run_pipeline_planners(
                    app_handle.clone(), conv_id.clone(), ws.clone(),
                    setup.planners, user_prompt, setup.abort.clone(),
                    setup.score_id_slots[..planner_count].to_vec(),
                    Some(prev), setup.stage_buffers[..planner_count].to_vec(),
                )
                .await;

                let merge_result = if planner_result.is_ok()
                    && !setup.abort.load(Ordering::Acquire)
                {
                    run_merge_chain(
                        app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                        setup.merge_agent, planner_count,
                        &setup.score_id_slots, &setup.stage_buffers,
                    )
                    .await
                } else {
                    None
                };

                let (status, error) = determine_final_status(&setup.abort, &planner_result, &merge_result);
                emit_final_status(&app_handle, &ws, &conv_id, status, error);
            }
            ResumePoint::PlanMerge => {
                re_emit_completed_stages(&app_handle, &conv_id, &ws, planner_count);

                let merge_result = run_merge_chain(
                    app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                    setup.merge_agent, planner_count,
                    &setup.score_id_slots, &setup.stage_buffers,
                )
                .await;

                let planner_ok: Result<(), String> = Ok(());
                let (status, error) = determine_final_status(&setup.abort, &planner_ok, &merge_result);
                emit_final_status(&app_handle, &ws, &conv_id, status, error);
            }
            ResumePoint::CodingPhase => {
                re_emit_completed_stages(&app_handle, &conv_id, &ws, indices_coder);

                let (status, error) = run_coding_phase(
                    app_handle.clone(), conv_id.clone(), ws.clone(), &setup, None,
                )
                .await;

                emit_final_status(&app_handle, &ws, &conv_id, status, error);
            }
            ResumePoint::Reviewers => {
                re_emit_completed_stages(&app_handle, &conv_id, &ws, setup.indices.reviewer_start);

                let (status, error) = run_coding_phase(
                    app_handle.clone(), conv_id.clone(), ws.clone(), &setup, Some(prev),
                )
                .await;

                emit_final_status(&app_handle, &ws, &conv_id, status, error);
            }
            ResumePoint::ReviewMerge => {
                re_emit_completed_stages(&app_handle, &conv_id, &ws, setup.indices.review_merge);

                let Some(review_merge_agent) = setup.reviewers.first().cloned() else {
                    emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed,
                        Some("No reviewer available for Review Merge".to_string()));
                    pipeline_cleanup(&ws, &conv_id);
                    return;
                };

                let rm_result = run_review_merge_chain(
                    app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                    review_merge_agent, &setup.indices,
                    &setup.score_id_slots, &setup.stage_buffers,
                )
                .await;

                if setup.abort.load(Ordering::Acquire) {
                    emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Stopped, None);
                } else if let Some(Err((_, e))) = &rm_result {
                    emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed, Some(e.clone()));
                } else if rm_result.is_none() {
                    emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed,
                        Some("Review merge skipped — no session ref".to_string()));
                } else {
                    // Proceed to Code Fixer.
                    let loaded = persistence::load_pipeline_state(&ws, &conv_id).ok().flatten();
                    let coder_ref = loaded.as_ref()
                        .and_then(|s| s.stages.iter().find(|st| st.stage_name == "Coder"))
                        .and_then(|st| st.provider_session_ref.clone())
                        .unwrap_or_default();

                    if coder_ref.is_empty() {
                        emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed,
                            Some("No coder session ref for Code Fixer".to_string()));
                    } else {
                        let fixer_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
                        ensure_stage_record(
                            &ws, &conv_id, setup.indices.code_fixer, "Code Fixer", &fixer_label,
                        );
                        let fixer_slot = setup.score_id_slots.get(setup.indices.code_fixer).cloned().unwrap_or_default();
                        let fixer_buf = setup.stage_buffers.get(setup.indices.code_fixer).cloned().unwrap_or_default();

                        let fixer_result = pipeline::run_code_fixer(
                            app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                            fixer_slot, fixer_buf, setup.indices.code_fixer, coder_ref, setup.coder.clone(),
                            None, None, None,
                        )
                        .await;

                        if setup.abort.load(Ordering::Acquire) {
                            emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Stopped, None);
                        } else {
                            match fixer_result {
                                Ok(_) => emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Completed, None),
                                Err((_, e)) => emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed, Some(e)),
                            }
                        }
                    }
                }
            }
            ResumePoint::CodeFixer => {
                re_emit_completed_stages(&app_handle, &conv_id, &ws, setup.indices.code_fixer);

                let loaded = persistence::load_pipeline_state(&ws, &conv_id).ok().flatten();
                let coder_ref = loaded.as_ref()
                    .and_then(|s| s.stages.iter().find(|st| st.stage_name == "Coder"))
                    .and_then(|st| st.provider_session_ref.clone())
                    .unwrap_or_default();

                if coder_ref.is_empty() {
                    emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed,
                        Some("No coder session ref for Code Fixer".to_string()));
                } else {
                    let fixer_label = format!("{} / {}", setup.coder.provider, setup.coder.model);
                    ensure_stage_record(
                        &ws, &conv_id, setup.indices.code_fixer, "Code Fixer", &fixer_label,
                    );
                    let fixer_slot = setup.score_id_slots.get(setup.indices.code_fixer).cloned().unwrap_or_default();
                    let fixer_buf = setup.stage_buffers.get(setup.indices.code_fixer).cloned().unwrap_or_default();

                    let fixer_result = pipeline::run_code_fixer(
                        app_handle.clone(), conv_id.clone(), ws.clone(), setup.abort.clone(),
                        fixer_slot, fixer_buf, setup.indices.code_fixer, coder_ref, setup.coder.clone(),
                        None, None, None,
                    )
                    .await;

                    if setup.abort.load(Ordering::Acquire) {
                        emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Stopped, None);
                    } else {
                        match fixer_result {
                            Ok(_) => emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Completed, None),
                            Err((_, e)) => emit_final_status(&app_handle, &ws, &conv_id, ConversationStatus::Failed, Some(e)),
                        }
                    }
                }
            }
        }

        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(detail)
}
