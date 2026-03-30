mod prompts;
pub mod stage_runner;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tauri::AppHandle;

use crate::models::PipelineAgent;
use crate::models::{ConversationStatus, PipelineStageRecord, PipelineState};
use crate::storage::now_rfc3339;

use prompts::{
    agent_label, build_plan_edit_prompt, build_plan_merge_prompt, build_planner_prompt,
};
use stage_runner::{emit_stage_status, run_stage, StageConfig};

/// Run the plan-merge stage with user feedback.
pub async fn run_plan_merge_with_feedback(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    job_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    feedback: String,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    run_plan_merge_inner(
        app, conversation_id, workspace_path, abort, job_id_slot,
        output_buffer, planner_count, provider_session_ref, agent,
        Some(feedback),
    )
    .await
}

/// Run the plan-merge stage. Resumes the first planner's hive-api session
/// and instructs it to read all individual plans and produce a merged plan.
pub async fn run_plan_merge(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    abort: Arc<AtomicBool>,
    job_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    run_plan_merge_inner(
        app, conversation_id, workspace_path, abort, job_id_slot,
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
    job_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
    planner_count: usize,
    provider_session_ref: String,
    agent: PipelineAgent,
    feedback: Option<String>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let stage_index = planner_count;
    let label = agent_label(&agent);

    let conv_dir = format!("{workspace_path}/.ea-code/conversations/{conversation_id}");
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
        },
        abort,
        job_id_slot,
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
    job_id_slots: Vec<Arc<std::sync::Mutex<Option<String>>>>,
    previous_stages: Option<Vec<PipelineStageRecord>>,
    stage_buffers: Vec<Arc<std::sync::Mutex<String>>>,
) -> Result<(), String> {
    let conv_dir = format!("{workspace_path}/.ea-code/conversations/{conversation_id}");

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
                    job_id: None,
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

        let job_slot = job_id_slots.get(i).cloned().unwrap_or_default();
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
