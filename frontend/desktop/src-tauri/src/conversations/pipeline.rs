use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::time::{sleep, Duration};

use crate::commands::api_health::hive_api_base_url;
use crate::models::PipelineAgent;
use crate::models::{
    ConversationStatus, PipelineStageOutputDelta, PipelineStageRecord, PipelineStageStatusEvent,
    PipelineState,
};
use crate::storage::now_rfc3339;

use super::events::{EVENT_PIPELINE_STAGE_OUTPUT_DELTA, EVENT_PIPELINE_STAGE_STATUS};
use super::sse::{consume_hive_sse, HiveSseEvent};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct HiveChatRequest<'a> {
    provider: &'a str,
    model: &'a str,
    workspace_path: &'a str,
    mode: &'a str,
    prompt: &'a str,
    provider_session_ref: Option<&'a str>,
    stream: bool,
    provider_options: HashMap<String, String>,
}

fn shared_client() -> &'static reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .build()
            .expect("failed to build client")
    })
}

fn request_hive_stop(job_id: String) {
    let client = shared_client().clone();
    let url = format!("{}/v1/chat/{job_id}/stop", hive_api_base_url());
    tokio::spawn(async move {
        match client.post(&url).send().await {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                eprintln!("[pipeline] Failed to stop hive job {job_id}: HTTP {status}: {body}");
            }
            Err(error) => eprintln!("[pipeline] Failed to stop hive job {job_id}: {error}"),
        }
    });
}

fn emit_stage_status(
    app: &AppHandle,
    conversation_id: &str,
    stage_index: usize,
    stage_name: &str,
    status: ConversationStatus,
    agent_label: &str,
) -> Result<(), String> {
    app.emit(
        EVENT_PIPELINE_STAGE_STATUS,
        PipelineStageStatusEvent {
            conversation_id: conversation_id.to_string(),
            stage_index,
            stage_name: stage_name.to_string(),
            status,
            agent_label: agent_label.to_string(),
        },
    )
    .map_err(|e| format!("Failed to emit pipeline stage status: {e}"))
}

fn emit_stage_delta(
    app: &AppHandle,
    conversation_id: &str,
    stage_index: usize,
    text: &str,
) -> Result<(), String> {
    app.emit(
        EVENT_PIPELINE_STAGE_OUTPUT_DELTA,
        PipelineStageOutputDelta {
            conversation_id: conversation_id.to_string(),
            stage_index,
            text: text.to_string(),
        },
    )
    .map_err(|e| format!("Failed to emit pipeline stage delta: {e}"))
}

fn build_planner_prompt(planner_number: usize, plan_dir: &str, user_prompt: &str) -> String {
    format!(
        "You are Planner {n} in a multi-agent code pipeline. Your task is to create a detailed implementation plan.\n\n\
         IMPORTANT RULES:\n\
         - Create ONLY a plan. Do NOT write any code.\n\
         - Save your plan as a markdown file to: {dir}/Plan-{n}.md\n\
         - Structure the plan with clear phases, numbered steps, and file paths where applicable.\n\
         - Focus on architecture, data flow, and implementation order.\n\
         - Be specific about which files to create/modify and what each should contain.\n\n\
         USER REQUEST:\n{prompt}",
        n = planner_number,
        dir = plan_dir,
        prompt = user_prompt,
    )
}

fn agent_label(agent: &PipelineAgent) -> String {
    format!("{} / {}", agent.provider, agent.model)
}

async fn run_single_planner(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    stage_index: usize,
    planner_number: usize,
    agent: PipelineAgent,
    plan_dir: String,
    user_prompt: String,
    abort: Arc<AtomicBool>,
    job_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    resume_session_ref: Option<String>,
    output_buffer: Arc<std::sync::Mutex<String>>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let stage_name = format!("Planner {planner_number}");
    let label = agent_label(&agent);
    let started_at = now_rfc3339();

    if let Err(e) = emit_stage_status(
        &app,
        &conversation_id,
        stage_index,
        &stage_name,
        ConversationStatus::Running,
        &label,
    ) {
        let record = PipelineStageRecord {
            stage_index,
            stage_name,
            agent_label: label,
            status: ConversationStatus::Failed,
            text: String::new(),
            started_at: Some(started_at),
            finished_at: Some(now_rfc3339()),
            job_id: None,
            provider_session_ref: None,
        };
        return Err((record, e));
    }

    let prompt = build_planner_prompt(planner_number, &plan_dir, &user_prompt);
    let mode = if resume_session_ref.is_some() {
        "resume"
    } else {
        "new"
    };
    let request = HiveChatRequest {
        provider: &agent.provider,
        model: &agent.model,
        workspace_path: &workspace_path,
        mode,
        prompt: &prompt,
        provider_session_ref: resume_session_ref.as_deref(),
        stream: true,
        provider_options: HashMap::new(),
    };

    let make_failed_record = |err_msg: &str| -> (PipelineStageRecord, String) {
        let _ = emit_stage_status(
            &app,
            &conversation_id,
            stage_index,
            &stage_name,
            ConversationStatus::Failed,
            &label,
        );
        (
            PipelineStageRecord {
                stage_index,
                stage_name: stage_name.clone(),
                agent_label: label.clone(),
                status: ConversationStatus::Failed,
                text: String::new(),
                started_at: Some(started_at.clone()),
                finished_at: Some(now_rfc3339()),
                job_id: None,
                provider_session_ref: None,
            },
            err_msg.to_string(),
        )
    };

    let url = format!("{}/v1/chat", hive_api_base_url());
    let response = match shared_client().post(&url).json(&request).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(make_failed_record(&format!(
                "Planner {planner_number} failed to contact hive-api: {e}"
            )));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(make_failed_record(&format!(
            "Planner {planner_number}: hive-api HTTP {status}: {body}"
        )));
    }

    // Watch for the plan file in a background task. Once it appears,
    // set an internal flag so the SSE loop knows the agent is done
    // even if the CLI keeps the session open.
    let plan_file = format!("{plan_dir}/Plan-{planner_number}.md");
    let file_ready = Arc::new(AtomicBool::new(false));
    let file_ready_writer = file_ready.clone();
    let plan_file_clone = plan_file.clone();
    // Per-planner stop signal. The file watcher sets THIS instead of the
    // shared `abort` so that sibling planners keep running.
    let planner_stop = Arc::new(AtomicBool::new(false));
    let planner_stop_writer = planner_stop.clone();
    let global_abort_for_watcher = abort.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(2)).await;
            if global_abort_for_watcher.load(Ordering::Acquire) {
                return;
            }
            if Path::new(&plan_file_clone).exists() {
                // Give the agent a moment to flush remaining output.
                sleep(Duration::from_secs(3)).await;
                file_ready_writer.store(true, Ordering::Release);
                planner_stop_writer.store(true, Ordering::Release);
                return;
            }
        }
    });

    // Propagate the global abort to this planner's local stop signal so
    // consume_hive_sse exits when the user presses Stop.
    let planner_stop_monitor = planner_stop.clone();
    let abort_monitor = abort.clone();
    tokio::spawn(async move {
        while !planner_stop_monitor.load(Ordering::Acquire) {
            if abort_monitor.load(Ordering::Acquire) {
                planner_stop_monitor.store(true, Ordering::Release);
                return;
            }
            sleep(Duration::from_millis(200)).await;
        }
    });

    let app_ref = app.clone();
    let conv_id = conversation_id.clone();
    let job_id_writer = job_id_slot.clone();
    let buf_writer = output_buffer.clone();
    let abort_for_run_started = abort.clone();
    let planner_stop_for_run_started = planner_stop.clone();
    let session_ref: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
    let session_ref_writer = session_ref.clone();
    let result = consume_hive_sse(response, &planner_stop, |event| match event {
        HiveSseEvent::OutputDelta { text } => {
            // Accumulate in the in-memory buffer so the text survives
            // frontend navigation (React state is cleared on route change).
            if let Ok(mut buf) = buf_writer.lock() {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(text);
            }
            emit_stage_delta(&app_ref, &conv_id, stage_index, text)
        }
        HiveSseEvent::RunStarted { job_id } => {
            if let Ok(mut guard) = job_id_writer.lock() {
                *guard = Some(job_id.clone());
            }
            if abort_for_run_started.load(Ordering::Acquire) {
                planner_stop_for_run_started.store(true, Ordering::Release);
                request_hive_stop(job_id.clone());
            }
            Ok(())
        }
        HiveSseEvent::ProviderSession {
            provider_session_ref,
        } => {
            if let Ok(mut guard) = session_ref_writer.lock() {
                *guard = Some(provider_session_ref.clone());
            }
            Ok(())
        }
        _ => Ok(()),
    })
    .await;

    let plan_created = file_ready.load(Ordering::Acquire) || Path::new(&plan_file).exists();

    let final_status = if abort.load(Ordering::Acquire) {
        ConversationStatus::Stopped
    } else if plan_created {
        // Plan file exists — agent did its job regardless of stream state.
        ConversationStatus::Completed
    } else {
        match &result {
            Ok(r) => r.status.clone(),
            Err(_) => ConversationStatus::Failed,
        }
    };

    let _ = emit_stage_status(
        &app,
        &conversation_id,
        stage_index,
        &stage_name,
        final_status.clone(),
        &label,
    );

    let captured_session_ref = session_ref.lock().ok().and_then(|g| g.clone());
    let captured_job = job_id_slot.lock().ok().and_then(|g| g.clone());
    let accumulated_text = output_buffer
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let record = PipelineStageRecord {
        stage_index,
        stage_name: stage_name.clone(),
        agent_label: label.clone(),
        status: final_status.clone(),
        text: accumulated_text,
        started_at: Some(started_at),
        finished_at: Some(now_rfc3339()),
        job_id: captured_job,
        provider_session_ref: captured_session_ref,
    };

    // Persist this stage immediately so the state is visible if the user
    // navigates away before all planners finish.
    if let Err(e) = super::persistence::update_pipeline_stage(
        &workspace_path,
        &conversation_id,
        &record,
    ) {
        eprintln!("[pipeline] Failed to save stage state for {stage_name}: {e}");
    }

    if final_status == ConversationStatus::Failed {
        Err((
            record,
            format!("Planner {planner_number} did not produce a plan"),
        ))
    } else {
        Ok(record)
    }
}

/// Run all planners in parallel. Returns when all planners have completed.
///
/// * `job_id_slots` — pre-allocated slots (one per planner) that receive the
///   hive-api job ID once `RunStarted` arrives. Registered in the global
///   pipeline-job registry so `stop_pipeline` can cancel them.
/// * `previous_stages` — when resuming a stopped/failed pipeline, pass the
///   saved stage records so that completed planners are skipped and
///   failed/stopped ones are resumed with their `provider_session_ref`.
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

    // Build the initial pipeline state. Completed stages from a previous run
    // keep their status; everything else is marked Running.
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

    // Spawn tasks only for planners that still need to run. Already-completed
    // stages are carried through directly.
    let mut spawned_indices: Vec<usize> = Vec::new();
    let mut handles = Vec::new();
    let mut completed_records: Vec<PipelineStageRecord> = Vec::new();

    for (i, agent) in planners.into_iter().enumerate() {
        let already_completed = previous_stages
            .as_ref()
            .and_then(|s| s.get(i))
            .map(|s| s.status == ConversationStatus::Completed)
            .unwrap_or(false);

        if already_completed {
            if let Some(record) = previous_stages.as_ref().and_then(|s| s.get(i)) {
                completed_records.push(record.clone());
            }
            continue;
        }

        // For failed/stopped stages, carry over the provider_session_ref so
        // the planner can resume its hive session.
        let resume_ref = previous_stages
            .as_ref()
            .and_then(|s| s.get(i))
            .and_then(|s| s.provider_session_ref.clone());

        let job_slot = job_id_slots.get(i).cloned().unwrap_or_default();
        let out_buf = stage_buffers.get(i).cloned().unwrap_or_default();
        let app = app.clone();
        let conv_id = conversation_id.clone();
        let ws = workspace_path.clone();
        let dir = plan_dir.clone();
        let prompt = user_prompt.clone();
        let abort = abort.clone();
        let planner_number = i + 1;

        spawned_indices.push(i);
        handles.push(tokio::spawn(async move {
            run_single_planner(
                app,
                conv_id,
                ws,
                i,
                planner_number,
                agent,
                dir,
                prompt,
                abort,
                job_slot,
                resume_ref,
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
                stage_records.push(PipelineStageRecord {
                    stage_index: stage_idx,
                    stage_name: format!("Planner {}", stage_idx + 1),
                    agent_label: String::new(),
                    status: ConversationStatus::Failed,
                    text: String::new(),
                    started_at: None,
                    finished_at: Some(now_rfc3339()),
                    job_id: None,
                    provider_session_ref: None,
                });
                errors.push(format!("Planner {} panicked: {e}", stage_idx + 1));
            }
        }
    }

    // Sort by stage_index so the order is stable regardless of completion order.
    stage_records.sort_by_key(|s| s.stage_index);

    // Persist pipeline state so it survives crashes.
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
