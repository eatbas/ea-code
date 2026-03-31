mod prompts;
pub mod stage_runner;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::PipelineAgent;
use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};
use crate::storage::now_rfc3339;

use prompts::{
    agent_label, build_code_fixer_prompt, build_coder_prompt, build_plan_edit_prompt,
    build_plan_merge_prompt, build_planner_prompt, build_review_merge_prompt,
    build_reviewer_prompt,
};
use stage_runner::{emit_stage_status, run_stage, StageConfig};

/// Run the plan-merge stage with user feedback.
pub async fn run_plan_merge_with_feedback(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    feedback: String,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    run_plan_merge_inner(
        app, conversation_id, workspace_path, abort, score_id_slot,
        output_buffer, planner_count, provider_session_ref, agent,
        Some(feedback),
    )
    .await
}

/// Run the plan-merge stage. Resumes the first planner's Symphony session
/// and instructs it to read all individual plans and produce a merged plan.
pub async fn run_plan_merge(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    run_plan_merge_inner(
        app, conversation_id, workspace_path, abort, score_id_slot,
        output_buffer, planner_count, provider_session_ref, agent,
        None,
    )
    .await
}

async fn run_plan_merge_inner(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    feedback: Option<String>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let stage_index = planner_count;
    let label = agent_label(&agent);

    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let plan_dir = format!("{conv_dir}/plan");
    let merged_dir = format!("{conv_dir}/plan_merged");

    if let Err(e) = std::fs::create_dir_all(&merged_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index, "Plan Merge".to_string(), label, Some(now_rfc3339()),
            ),
            format!("Failed to create plan_merged directory: {e}"),
        ));
    }

    // When editing, remove the old merged plan so the file watcher can
    // detect when the agent writes a fresh version.
    if feedback.is_some() {
        let old_file = format!("{merged_dir}/plan_merged.md");
        let _ = std::fs::remove_file(&old_file);
    }

    let prompt = if let Some(ref fb) = feedback {
        build_plan_edit_prompt(fb, &merged_dir)
    } else {
        build_plan_merge_prompt(planner_count, &plan_dir, &merged_dir)
    };

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name: "Plan Merge".to_string(),
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{merged_dir}/plan_merged.md"),
            mode: "resume",
            provider_session_ref: Some(provider_session_ref),
            failure_message: "Plan Merge did not produce a merged plan".to_string(),
            agent_label: label,
            file_required: true,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}

/// Run all planners in parallel. Returns when all planners have completed.
pub async fn run_pipeline_planners(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    planners: Vec<PipelineAgent>,
    user_prompt: String,
    abort: Arc<AtomicBool>,
    score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    previous_stages: Option<Vec<PipelineStageRecord>>,
    stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
) -> Result<(), String> {
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");

    // Save user prompt in its own folder.
    let prompt_dir = format!("{conv_dir}/prompt");
    std::fs::create_dir_all(&prompt_dir)
        .map_err(|e| format!("Failed to create prompt directory: {e}"))?;
    std::fs::write(format!("{prompt_dir}/prompt.md"), &user_prompt)
        .map_err(|e| format!("Failed to save prompt: {e}"))?;

    // Create the plan folder for planner outputs.
    let plan_dir = format!("{conv_dir}/plan");
    std::fs::create_dir_all(&plan_dir)
        .map_err(|e| format!("Failed to create plan directory: {e}"))?;

    // Build the initial pipeline state.
    let initial_stages: Vec<PipelineStageRecord> = planners
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let already_completed = previous_stages
                .as_ref()
                .and_then(|s| s.get(i))
                .map(|s| s.status == ConversationStatus::Completed)
                .unwrap_or(false);

            if already_completed {
                previous_stages.as_ref().unwrap()[i].clone()
            } else {
                PipelineStageRecord {
                    stage_index: i,
                    stage_name: format!("Planner {}", i + 1),
                    agent_label: agent_label(a),
                    status: ConversationStatus::Running,
                    text: String::new(),
                    started_at: Some(now_rfc3339()),
                    finished_at: None,
                    score_id: None,
                    provider_session_ref: None,
                }
            }
        })
        .collect();
    let initial_state = PipelineState {
        user_prompt: user_prompt.clone(),
        pipeline_mode: "code".to_string(),
        stages: initial_stages,
    };
    if let Err(e) =
        super::persistence::save_pipeline_state(&workspace_path, &conversation_id, &initial_state)
    {
        eprintln!("[pipeline] Failed to save initial pipeline state: {e}");
    }

    let planner_count = planners.len();
    let mut spawned_indices: Vec<usize> = Vec::new();
    let mut handles = Vec::new();
    let mut completed_records: Vec<PipelineStageRecord> = Vec::new();

    for (i, planner_agent) in planners.into_iter().enumerate() {
        let already_completed = previous_stages
            .as_ref()
            .and_then(|s| s.get(i))
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false);

        if already_completed {
            if let Some(record) = previous_stages.as_ref().and_then(|s| s.get(i)) {
                let _ = emit_stage_status(
                    &app, &conversation_id, i, &record.stage_name,
                    record.status.clone(), &record.agent_label,
                    if record.text.is_empty() { None } else { Some(record.text.clone()) },
                );
                completed_records.push(record.clone());
            }
            continue;
        }

        let resume_ref = previous_stages
            .as_ref()
            .and_then(|s| s.get(i))
            .and_then(|s| s.provider_session_ref.clone());

        let job_slot = score_id_slots.get(i).cloned().unwrap_or_default();
        let out_buf = stage_buffers.get(i).cloned().unwrap_or_default();
        let app_c = app.clone();
        let conv_id = conversation_id.clone();
        let ws = workspace_path.clone();
        let dir = plan_dir.clone();
        let prompt_text = user_prompt.clone();
        let abort_c = abort.clone();
        let planner_number = i + 1;
        let label = agent_label(&planner_agent);
        let mode = if resume_ref.is_some() { "resume" } else { "new" };

        spawned_indices.push(i);
        handles.push(tokio::spawn(async move {
            run_stage(
                app_c,
                conv_id,
                ws,
                StageConfig {
                    stage_index: i,
                    stage_name: format!("Planner {planner_number}"),
                    provider: planner_agent.provider,
                    model: planner_agent.model,
                    prompt: build_planner_prompt(planner_number, &dir, &prompt_text),
                    file_to_watch: format!("{dir}/Plan-{planner_number}.md"),
                    mode,
                    provider_session_ref: resume_ref,
                    failure_message: format!("Planner {planner_number} did not produce a plan"),
                    agent_label: label,
                    file_required: true,
                },
                abort_c,
                job_slot,
                out_buf,
            )
            .await
        }));
    }

    let results = futures::future::join_all(handles).await;
    let mut stage_records: Vec<PipelineStageRecord> = completed_records;
    stage_records.reserve(planner_count);
    let mut errors = Vec::new();

    for (result_idx, result) in results.into_iter().enumerate() {
        let stage_idx = spawned_indices[result_idx];
        match result {
            Ok(Ok(record)) => stage_records.push(record),
            Ok(Err((record, e))) => {
                stage_records.push(record);
                errors.push(format!("Planner {}: {e}", stage_idx + 1));
            }
            Err(e) => {
                stage_records.push(PipelineStageRecord::failed(
                    stage_idx,
                    format!("Planner {}", stage_idx + 1),
                    String::new(),
                    None,
                ));
                errors.push(format!("Planner {} panicked: {e}", stage_idx + 1));
            }
        }
    }

    stage_records.sort_by_key(|s| s.stage_index);

    let state = PipelineState {
        user_prompt,
        pipeline_mode: "code".to_string(),
        stages: stage_records,
    };
    if let Err(e) =
        super::persistence::save_pipeline_state(&workspace_path, &conversation_id, &state)
    {
        eprintln!("[pipeline] Failed to save pipeline state: {e}");
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Run the single Coder stage. Starts a new session and implements the
/// approved merged plan.
pub async fn run_coder(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let coder_dir = format!("{conv_dir}/coder");
    let merged_plan = format!("{conv_dir}/plan_merged/plan_merged.md");

    if let Err(e) = std::fs::create_dir_all(&coder_dir) {
        return Err((
            PipelineStageRecord::failed(stage_index, "Coder".to_string(), label, Some(now_rfc3339())),
            format!("Failed to create coder directory: {e}"),
        ));
    }

    let prompt = build_coder_prompt(&merged_plan, &coder_dir);

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name: "Coder".to_string(),
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{coder_dir}/coder_done.md"),
            mode: "new",
            provider_session_ref: None,
            failure_message: "Coder did not produce a completion summary".to_string(),
            agent_label: label,
            file_required: false,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}

/// Run all reviewers in parallel. Each reviewer resumes the corresponding
/// planner's session to retain plan context, then uses git tools to review
/// the coder's changes.
pub async fn run_pipeline_reviewers(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    reviewers: Vec<PipelineAgent>,
    abort: Arc<AtomicBool>,
    score_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    previous_stages: Option<Vec<PipelineStageRecord>>,
    stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
    planner_stages: &[PipelineStageRecord],
    reviewer_start_index: usize,
) -> Result<(), String> {
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let review_dir = format!("{conv_dir}/review");
    let plan_merged_path = format!("{conv_dir}/plan_merged/plan_merged.md");

    std::fs::create_dir_all(&review_dir)
        .map_err(|e| format!("Failed to create review directory: {e}"))?;

    let mut spawned_indices: Vec<usize> = Vec::new();
    let mut handles = Vec::new();

    for (i, reviewer_agent) in reviewers.into_iter().enumerate() {
        let stage_idx = reviewer_start_index + i;
        let reviewer_number = i + 1;
        let label = agent_label(&reviewer_agent);

        let already_completed = previous_stages
            .as_ref()
            .and_then(|s| s.iter().find(|st| st.stage_index == stage_idx))
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false);

        if already_completed {
            if let Some(record) = previous_stages
                .as_ref()
                .and_then(|s| s.iter().find(|st| st.stage_index == stage_idx))
            {
                let _ = emit_stage_status(
                    &app, &conversation_id, stage_idx, &record.stage_name,
                    record.status.clone(), &record.agent_label,
                    if record.text.is_empty() { None } else { Some(record.text.clone()) },
                );
            }
            continue;
        }

        let planner_session_ref = match planner_stages.get(i) {
            Some(stage) => match stage.provider_session_ref.clone().filter(|value| !value.is_empty()) {
                Some(session_ref) => session_ref,
                None => {
                    let failed_record = PipelineStageRecord::failed(
                        stage_idx,
                        format!("Reviewer {reviewer_number}"),
                        label.clone(),
                        Some(now_rfc3339()),
                    );
                    let _ = crate::conversations::persistence::update_pipeline_stage(
                        &workspace_path,
                        &conversation_id,
                        &failed_record,
                    );
                    let _ = emit_stage_status(
                        &app,
                        &conversation_id,
                        stage_idx,
                        &failed_record.stage_name,
                        ConversationStatus::Failed,
                        &label,
                        None,
                    );
                    return Err(format!(
                        "Reviewer {reviewer_number}: Planner {reviewer_number} is missing a provider session ref",
                    ));
                }
            },
            None => {
                let failed_record = PipelineStageRecord::failed(
                    stage_idx,
                    format!("Reviewer {reviewer_number}"),
                    label.clone(),
                    Some(now_rfc3339()),
                );
                let _ = crate::conversations::persistence::update_pipeline_stage(
                    &workspace_path,
                    &conversation_id,
                    &failed_record,
                );
                let _ = emit_stage_status(
                    &app,
                    &conversation_id,
                    stage_idx,
                    &failed_record.stage_name,
                    ConversationStatus::Failed,
                    &label,
                    None,
                );
                return Err(format!(
                    "Reviewer {reviewer_number}: missing matching Planner {reviewer_number} stage",
                ));
            }
        };

        let job_slot = score_id_slots.get(i).cloned().unwrap_or_default();
        let out_buf = stage_buffers.get(i).cloned().unwrap_or_default();
        let app_c = app.clone();
        let conv_id = conversation_id.clone();
        let ws = workspace_path.clone();
        let dir = review_dir.clone();
        let plan_path = plan_merged_path.clone();
        let abort_c = abort.clone();

        spawned_indices.push(stage_idx);
        handles.push(tokio::spawn(async move {
            run_stage(
                app_c,
                conv_id,
                ws,
                StageConfig {
                    stage_index: stage_idx,
                    stage_name: format!("Reviewer {reviewer_number}"),
                    provider: reviewer_agent.provider,
                    model: reviewer_agent.model,
                    prompt: build_reviewer_prompt(reviewer_number, &plan_path, &dir),
                    file_to_watch: format!("{dir}/Review-{reviewer_number}.md"),
                    mode: "resume",
                    provider_session_ref: Some(planner_session_ref),
                    failure_message: format!("Reviewer {reviewer_number} did not produce a review"),
                    agent_label: label,
                    file_required: true,
                },
                abort_c,
                job_slot,
                out_buf,
            )
            .await
        }));
    }

    let results = futures::future::join_all(handles).await;
    let mut errors = Vec::new();

    for (result_idx, result) in results.into_iter().enumerate() {
        let stage_idx = spawned_indices[result_idx];
        match result {
            Ok(Ok(_record)) => {}
            Ok(Err((_record, e))) => {
                errors.push(format!("Reviewer {}: {e}", stage_idx - reviewer_start_index + 1));
            }
            Err(e) => {
                errors.push(format!(
                    "Reviewer {} panicked: {e}",
                    stage_idx - reviewer_start_index + 1
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Run the review-merge stage. Resumes the first reviewer's session
/// and instructs it to read all individual reviews and produce a
/// consolidated review.
pub async fn run_review_merge(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    reviewer_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let review_dir = format!("{conv_dir}/review");
    let review_merged_dir = format!("{conv_dir}/review_merged");

    if let Err(e) = std::fs::create_dir_all(&review_merged_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index, "Review Merge".to_string(), label, Some(now_rfc3339()),
            ),
            format!("Failed to create review_merged directory: {e}"),
        ));
    }

    let prompt = build_review_merge_prompt(reviewer_count, &review_dir, &review_merged_dir);

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name: "Review Merge".to_string(),
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{review_merged_dir}/review_merged.md"),
            mode: "resume",
            provider_session_ref: Some(provider_session_ref),
            failure_message: "Review Merge did not produce a merged review".to_string(),
            agent_label: label,
            file_required: true,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}

/// Run the Code Fixer stage. Resumes the coder's session and applies
/// fixes based on the consolidated review.
pub async fn run_code_fixer(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    stage_index: usize,
    coder_session_ref: String,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let label = agent_label(&agent);
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");
    let code_fixer_dir = format!("{conv_dir}/code_fixer");
    let review_merged_path = format!("{conv_dir}/review_merged/review_merged.md");

    if let Err(e) = std::fs::create_dir_all(&code_fixer_dir) {
        return Err((
            PipelineStageRecord::failed(
                stage_index, "Code Fixer".to_string(), label, Some(now_rfc3339()),
            ),
            format!("Failed to create code_fixer directory: {e}"),
        ));
    }

    let prompt = build_code_fixer_prompt(&review_merged_path, &code_fixer_dir);

    run_stage(
        app,
        conversation_id,
        workspace_path,
        StageConfig {
            stage_index,
            stage_name: "Code Fixer".to_string(),
            provider: agent.provider,
            model: agent.model,
            prompt,
            file_to_watch: format!("{code_fixer_dir}/code_fixer_done.md"),
            mode: "resume",
            provider_session_ref: Some(coder_session_ref),
            failure_message: "Code Fixer did not produce a completion summary".to_string(),
            agent_label: label,
            file_required: false,
        },
        abort,
        score_id_slot,
        output_buffer,
    )
    .await
}
