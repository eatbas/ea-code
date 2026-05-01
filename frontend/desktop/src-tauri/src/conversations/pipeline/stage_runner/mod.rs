mod emission;
mod finalise;
mod watchers;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::AppHandle;
use tokio::time::{sleep, Duration, Instant};

use crate::conversations::pipeline_debug::emit_pipeline_debug;
use crate::conversations::score_client::{
    consume_score_websocket, fetch_score_snapshot, SymphonyLiveEvent, SymphonyScoreSnapshot,
    SCORE_POLL_INTERVAL,
};
use crate::conversations::symphony_request::SymphonyChatRequest;
use crate::models::{ConversationStatus, PipelineStageRecord};
use crate::storage::now_rfc3339;

use self::emission::{emit_stage_delta, emit_stage_record_status, request_symphony_stop};
use self::finalise::{
    append_live_output, describe_stage_failure, determine_final_status, is_network_error,
    live_output, maybe_update_session_ref, resolve_stage_text, stage_error_text,
    sync_snapshot_output,
};
use self::watchers::spawn_stage_watchers;

const FILE_COMPLETION_STOP_GRACE: Duration = Duration::from_secs(15);
const POLL_FAILURE_GRACE: Duration = Duration::from_secs(45);

/// Maximum number of attempts (1 initial + 2 retries) for a single
/// pipeline stage when a transient network error is detected. After
/// this is exhausted the stage is reported as Failed.
const MAX_STAGE_ATTEMPTS: u32 = 3;
/// Backoff between auto-retry attempts. Long enough to let a brief
/// provider-side outage clear; short enough that a multi-stage pipeline
/// does not stall for minutes when the API is genuinely down.
const RETRY_BACKOFF: Duration = Duration::from_secs(10);
/// Prompt used when resuming a captured provider session for a retry.
/// Plain `continue` lets the agent pick up exactly where the stream was
/// interrupted without reframing the original task.
const RETRY_CONTINUE_PROMPT: &str = "continue";

/// Prompt prefix prepended to pipeline stage prompts when Kimi swarm mode
/// is active. Instructs the agent to create specialised subagents and
/// dispatch tasks in parallel.
const PIPELINE_SWARM_PREFIX: &str = "\
    [SWARM MODE] You have CreateSubagent and Task tools available. \
    Before starting work, analyse the project and create specialised \
    subagents (e.g. coder, reviewer, researcher, tester). Then use \
    Task to dispatch independent subtasks in parallel \u{2014} call Task \
    multiple times in a single response for maximum concurrency. \
    Aim for maximum parallelism. Spawn as many subagents as needed.";

/// Configuration for a single pipeline stage execution.
pub struct StageConfig {
    pub stage_index: usize,
    pub stage_name: String,
    pub provider: String,
    pub model: String,
    pub prompt: String,
    pub file_to_watch: String,
    pub mode: &'static str,
    pub provider_session_ref: Option<String>,
    pub failure_message: String,
    pub agent_label: String,
    /// When `false`, the stage completes based on the score status alone.
    /// Suitable for coding agents that modify the codebase but may not
    /// write a dedicated marker file.
    pub file_required: bool,
}

/// Outcome of a single attempt at running a stage. Used by the retry
/// loop to decide whether to re-attempt with `continue`.
struct AttemptOutcome {
    terminal_snapshot: Result<SymphonyScoreSnapshot, String>,
    final_status: ConversationStatus,
    captured_session_ref: Option<String>,
    captured_job: Option<String>,
    display_text: Option<String>,
    accumulated_text: String,
}

pub(super) fn emit_stage_status(
    app: &AppHandle,
    conversation_id: &str,
    stage_index: usize,
    stage_name: &str,
    status: ConversationStatus,
    agent_label: &str,
    text: Option<String>,
) -> Result<(), String> {
    emission::emit_stage_status(
        app,
        conversation_id,
        stage_index,
        stage_name,
        status,
        agent_label,
        text,
    )
}

pub(super) fn emit_stage_record(
    app: &AppHandle,
    conversation_id: &str,
    record: &PipelineStageRecord,
    text: Option<String>,
) -> Result<(), String> {
    emission::emit_stage_record_status(app, conversation_id, record, text)
}

fn persist_stage_progress(
    workspace_path: &str,
    conversation_id: &str,
    stage_index: usize,
    stage_name: &str,
    agent_label: &str,
    started_at: &str,
    output_buffer: &Arc<std::sync::Mutex<String>>,
    score_id: Option<String>,
    provider_session_ref: Option<String>,
) {
    let record = PipelineStageRecord {
        stage_index,
        stage_name: stage_name.to_string(),
        agent_label: agent_label.to_string(),
        status: ConversationStatus::Running,
        text: live_output(output_buffer),
        started_at: Some(started_at.to_string()),
        finished_at: None,
        score_id,
        provider_session_ref,
    };

    if let Err(error) = crate::conversations::persistence::update_pipeline_stage(
        workspace_path,
        conversation_id,
        &record,
    ) {
        eprintln!("[pipeline] Failed to persist stage progress for {stage_name}: {error}");
    }
}

/// Execute a single pipeline stage with auto-retry on transient
/// connection failures.
///
/// The first attempt uses the caller-supplied prompt and mode. If the
/// score completes with a network-class error and the run captured a
/// provider session ref, the stage is re-submitted in `resume` mode
/// with a `continue` prompt so the agent picks up where the interrupted
/// stream left off. Up to [`MAX_STAGE_ATTEMPTS`] attempts are made before
/// the stage is reported as Failed.
pub async fn run_stage(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    config: StageConfig,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let stage_index = config.stage_index;
    let stage_name = config.stage_name.clone();
    let provider = config.provider.clone();
    let model = config.model.clone();
    let initial_prompt = config.prompt.clone();
    let file_to_watch = config.file_to_watch.clone();
    let initial_mode = config.mode;
    let initial_session_ref = config.provider_session_ref.clone();
    let failure_message = config.failure_message.clone();
    let agent_label = config.agent_label.clone();
    let file_required = config.file_required;

    let started_at = now_rfc3339();

    if let Err(error) = emit_stage_status(
        &app,
        &conversation_id,
        stage_index,
        &stage_name,
        ConversationStatus::Running,
        &agent_label,
        None,
    ) {
        return Err((
            PipelineStageRecord::failed(
                stage_index,
                stage_name.clone(),
                agent_label.clone(),
                Some(started_at.clone()),
            ),
            error,
        ));
    }

    let mut current_mode: &'static str = initial_mode;
    let mut current_prompt = initial_prompt;
    let mut current_session_ref = initial_session_ref;
    let mut last_outcome: Option<AttemptOutcome> = None;

    for attempt in 0..MAX_STAGE_ATTEMPTS {
        let attempt_index = attempt + 1;
        if attempt_index > 1 {
            emit_pipeline_debug(
                &app,
                &workspace_path,
                &conversation_id,
                format!(
                    "{stage_name}: attempt {attempt_index}/{MAX_STAGE_ATTEMPTS} resuming session {} with `continue`",
                    current_session_ref.as_deref().unwrap_or("<none>"),
                ),
            );
        } else {
            emit_pipeline_debug(
                &app,
                &workspace_path,
                &conversation_id,
                format!(
                    "{stage_name}: starting stage with {provider}/{model}, mode={current_mode}, file_required={file_required}, file_to_watch={file_to_watch}",
                ),
            );
        }

        let attempt_outcome = attempt_stage(
            &app,
            &conversation_id,
            &workspace_path,
            stage_index,
            &stage_name,
            &provider,
            &model,
            &current_prompt,
            current_mode,
            current_session_ref.as_deref(),
            &file_to_watch,
            file_required,
            &agent_label,
            &started_at,
            abort.clone(),
            score_id_slot.clone(),
            output_buffer.clone(),
        )
        .await;

        // On a clean stop or non-failed terminal status we are done.
        if attempt_outcome.final_status != ConversationStatus::Failed {
            last_outcome = Some(attempt_outcome);
            break;
        }

        // The user may have aborted between attempts — respect that even
        // if the score itself reported Failed.
        if abort.load(Ordering::Acquire) {
            last_outcome = Some(attempt_outcome);
            break;
        }

        let error_text = stage_error_text(&attempt_outcome.terminal_snapshot).unwrap_or_default();
        let recoverable =
            !error_text.is_empty() && is_network_error(&error_text);
        let session_for_retry = attempt_outcome
            .captured_session_ref
            .clone()
            .or_else(|| current_session_ref.clone());

        let can_retry =
            recoverable && session_for_retry.is_some() && attempt_index < MAX_STAGE_ATTEMPTS;

        if !can_retry {
            if recoverable && session_for_retry.is_none() {
                emit_pipeline_debug(
                    &app,
                    &workspace_path,
                    &conversation_id,
                    format!(
                        "{stage_name}: detected network error but no provider session captured \u{2014} cannot retry",
                    ),
                );
            } else if recoverable {
                emit_pipeline_debug(
                    &app,
                    &workspace_path,
                    &conversation_id,
                    format!(
                        "{stage_name}: exhausted {MAX_STAGE_ATTEMPTS} attempts after network errors",
                    ),
                );
            }
            last_outcome = Some(attempt_outcome);
            break;
        }

        emit_pipeline_debug(
            &app,
            &workspace_path,
            &conversation_id,
            format!(
                "{stage_name}: attempt {attempt_index} hit network error ({}); retrying after {}s",
                truncate_for_log(&error_text, 200),
                RETRY_BACKOFF.as_secs(),
            ),
        );

        // Append a visible separator into the live output buffer so the
        // user can see in the stage transcript that a retry was issued
        // and what triggered it.
        append_live_output(
            &output_buffer,
            &format!(
                "\n--- retry: attempt {attempt_index} hit network error; resuming with `continue` ---\n",
            ),
        );
        if let Err(error) =
            emit_stage_delta(
                &app,
                &conversation_id,
                stage_index,
                &format!(
                    "\n--- retry: attempt {attempt_index} hit network error; resuming with `continue` ---\n",
                ),
            )
        {
            eprintln!("[pipeline] {stage_name}: failed to emit retry separator: {error}");
        }

        // Sleep with abort awareness so a stop click during backoff
        // exits promptly.
        let mut aborted_during_backoff = false;
        let backoff_deadline = Instant::now() + RETRY_BACKOFF;
        while Instant::now() < backoff_deadline {
            if abort.load(Ordering::Acquire) {
                aborted_during_backoff = true;
                break;
            }
            sleep(Duration::from_millis(200)).await;
        }

        if aborted_during_backoff {
            last_outcome = Some(attempt_outcome);
            break;
        }

        current_mode = "resume";
        current_prompt = RETRY_CONTINUE_PROMPT.to_string();
        current_session_ref = session_for_retry;
        last_outcome = Some(attempt_outcome);
    }

    let outcome = last_outcome.expect("at least one attempt always runs");
    finalise_stage_record(
        &app,
        &conversation_id,
        &workspace_path,
        stage_index,
        &stage_name,
        &agent_label,
        &started_at,
        &file_to_watch,
        file_required,
        &failure_message,
        outcome,
    )
}

/// Emit + persist the final stage record using the most recent attempt's
/// outcome. Returns `Ok` on a successful stage and `Err` with the
/// caller-supplied failure message otherwise.
#[allow(clippy::too_many_arguments)]
fn finalise_stage_record(
    app: &AppHandle,
    conversation_id: &str,
    workspace_path: &str,
    stage_index: usize,
    stage_name: &str,
    agent_label: &str,
    started_at: &str,
    file_to_watch: &str,
    file_required: bool,
    failure_message: &str,
    outcome: AttemptOutcome,
) -> Result<PipelineStageRecord, (PipelineStageRecord, String)> {
    let AttemptOutcome {
        terminal_snapshot,
        final_status,
        captured_session_ref,
        captured_job,
        display_text,
        accumulated_text,
    } = outcome;

    let failure_text = if final_status == ConversationStatus::Failed {
        Some(describe_stage_failure(
            stage_name,
            file_to_watch,
            file_required,
            &terminal_snapshot,
            captured_job.as_deref(),
            captured_session_ref.as_deref(),
            &accumulated_text,
        ))
    } else {
        None
    };

    let display_text = display_text.or_else(|| failure_text.clone());

    if let Some(ref diagnostic) = failure_text {
        eprintln!("[pipeline] {stage_name}: {diagnostic}");
        for line in diagnostic.lines() {
            emit_pipeline_debug(
                app,
                workspace_path,
                conversation_id,
                format!("{stage_name}: {line}"),
            );
        }
    }

    emit_pipeline_debug(
        app,
        workspace_path,
        conversation_id,
        format!(
            "{stage_name}: final status resolved to {:?}; watched_file_exists={}",
            final_status,
            std::path::Path::new(file_to_watch).exists(),
        ),
    );

    let record = PipelineStageRecord {
        stage_index,
        stage_name: stage_name.to_string(),
        agent_label: agent_label.to_string(),
        status: final_status.clone(),
        text: display_text.unwrap_or(accumulated_text),
        started_at: Some(started_at.to_string()),
        finished_at: Some(now_rfc3339()),
        score_id: captured_job,
        provider_session_ref: captured_session_ref,
    };

    let _ = emit_stage_record_status(app, conversation_id, &record, Some(record.text.clone()));

    if let Err(error) = crate::conversations::persistence::update_pipeline_stage(
        workspace_path,
        conversation_id,
        &record,
    ) {
        eprintln!("[pipeline] Failed to save stage state for {stage_name}: {error}");
    }

    if final_status == ConversationStatus::Failed {
        Err((record, failure_message.to_string()))
    } else {
        Ok(record)
    }
}

fn truncate_for_log(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let mut truncated: String = text.chars().take(max_chars).collect();
        truncated.push_str("...");
        truncated
    }
}

#[allow(clippy::too_many_arguments)]
async fn attempt_stage(
    app: &AppHandle,
    conversation_id: &str,
    workspace_path: &str,
    stage_index: usize,
    stage_name: &str,
    provider: &str,
    model: &str,
    prompt: &str,
    mode: &str,
    provider_session_ref: Option<&str>,
    file_to_watch: &str,
    file_required: bool,
    agent_label: &str,
    started_at: &str,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    output_buffer: Arc<std::sync::Mutex<String>>,
) -> AttemptOutcome {
    let settings = crate::storage::settings::read_settings().ok();
    let thinking_level = settings
        .as_ref()
        .and_then(|s| s.thinking_level(provider, model).map(str::to_string));
    // Kimi swarm spins up coding subagents that aggressively modify
    // files in parallel. That behaviour is exactly what the **Coder**
    // and **Code Fixer** stages need, but it is destructive everywhere
    // else: planners, reviewers, and merge stages are supposed to
    // analyse / plan / synthesise without touching the codebase. With
    // swarm enabled in those stages the planner subagents skip writing
    // their `Plan-N.md` artefact and start editing source files
    // directly -- exactly the symptom the user reported (planning
    // phase produced 1500+ lines of code diffs instead of a plan).
    //
    // Use an explicit allowlist of stage names rather than a blocklist
    // so any future read-only stage we add stays safe by default.
    let stage_allows_swarm = matches!(stage_name, "Coder" | "Code Fixer");
    let kimi_swarm = settings
        .as_ref()
        .filter(|_| mode == "new" && stage_allows_swarm)
        .filter(|s| provider.eq_ignore_ascii_case("kimi") && s.kimi_swarm_enabled)
        .map(|s| {
            let swarm_dir =
                format!("{workspace_path}/.maestro/conversations/{conversation_id}/swarm");
            let yaml_path = format!("{swarm_dir}/kimi-swarm.yaml");
            if !std::path::Path::new(&yaml_path).exists() {
                let _ = std::fs::create_dir_all(&swarm_dir);
                let _ = std::fs::write(&yaml_path, crate::models::KIMI_SWARM_YAML);
            }
            crate::conversations::symphony_request::KimiSwarmOptions {
                agent_file: yaml_path,
                swarm_dir,
                max_ralph_iterations: s.kimi_max_ralph_iterations,
            }
        });
    // Prepend swarm instructions when swarm mode is active.
    let augmented_prompt = if kimi_swarm.is_some() {
        format!("{PIPELINE_SWARM_PREFIX}\n\n{prompt}")
    } else {
        prompt.to_string()
    };
    let provider_options = crate::conversations::symphony_request::default_provider_options(
        provider,
        model,
        thinking_level.as_deref(),
        kimi_swarm,
    );
    emit_pipeline_debug(
        app,
        workspace_path,
        conversation_id,
        format!(
            "{stage_name}: options: {}",
            serde_json::to_string(&provider_options).unwrap_or_else(|_| "{}".to_string()),
        ),
    );
    let request = SymphonyChatRequest {
        provider,
        model,
        workspace_path,
        mode,
        prompt: &augmented_prompt,
        provider_session_ref,
        provider_options,
    };

    let accepted = match crate::conversations::score_client::submit_score(&request).await {
        Ok(response) => response,
        Err(error) => {
            emit_pipeline_debug(
                app,
                workspace_path,
                conversation_id,
                format!("{stage_name}: failed to submit score: {error}"),
            );
            // Report a Failed outcome with no captured session so the
            // retry loop classifies as network-class failure (the marker
            // text contains "Failed to submit Symphony score") and can
            // re-issue against the existing session_ref the caller
            // already has.
            return AttemptOutcome {
                terminal_snapshot: Err(error.clone()),
                final_status: ConversationStatus::Failed,
                captured_session_ref: provider_session_ref.map(str::to_string),
                captured_job: None,
                display_text: None,
                accumulated_text: live_output(&output_buffer),
            };
        }
    };
    emit_pipeline_debug(
        app,
        workspace_path,
        conversation_id,
        format!("{stage_name}: accepted score {}", accepted.score_id),
    );

    if let Ok(mut guard) = score_id_slot.lock() {
        *guard = Some(accepted.score_id.clone());
    }

    persist_stage_progress(
        workspace_path,
        conversation_id,
        stage_index,
        stage_name,
        agent_label,
        started_at,
        &output_buffer,
        Some(accepted.score_id.clone()),
        provider_session_ref.map(str::to_string),
    );

    let watchers = spawn_stage_watchers(file_to_watch.to_string(), abort.clone());
    let app_ref = app.clone();
    let conversation_id_ref = conversation_id.to_string();
    let output_buffer_writer = output_buffer.clone();
    let session_ref: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(
        provider_session_ref.map(str::to_string),
    ));
    let session_ref_writer = session_ref.clone();
    let websocket_stop = Arc::new(AtomicBool::new(false));

    {
        let websocket_stop_ref = websocket_stop.clone();
        let score_id = accepted.score_id.clone();
        let stage_name_for_ws = stage_name.to_string();
        let agent_label_for_ws = agent_label.to_string();
        let started_at_for_ws = started_at.to_string();
        let app_for_ws = app.clone();
        let workspace_for_ws = workspace_path.to_string();
        let conversation_for_ws = conversation_id.to_string();
        tokio::spawn(async move {
            let result =
                consume_score_websocket(
                    &score_id,
                    websocket_stop_ref.as_ref(),
                    |event| match event {
                        SymphonyLiveEvent::ScoreSnapshot(snapshot) => {
                            if let Some(delta) = sync_snapshot_output(
                                &output_buffer_writer,
                                &snapshot.accumulated_text,
                            ) {
                                emit_stage_delta(
                                    &app_ref,
                                    &conversation_id_ref,
                                    stage_index,
                                    &delta,
                                )?;
                            }
                            if let Some(provider_session_ref) = maybe_update_session_ref(
                                &session_ref_writer,
                                snapshot.provider_session_ref.as_deref(),
                            ) {
                                persist_stage_progress(
                                    &workspace_for_ws,
                                    &conversation_for_ws,
                                    stage_index,
                                    &stage_name_for_ws,
                                    &agent_label_for_ws,
                                    &started_at_for_ws,
                                    &output_buffer_writer,
                                    Some(score_id.clone()),
                                    Some(provider_session_ref),
                                );
                            }
                            Ok(())
                        }
                        SymphonyLiveEvent::OutputDelta { text } => {
                            append_live_output(&output_buffer_writer, &text);
                            emit_stage_delta(&app_ref, &conversation_id_ref, stage_index, &text)
                        }
                        SymphonyLiveEvent::ProviderSession {
                            provider_session_ref,
                        } => {
                            let updated_session = maybe_update_session_ref(
                                &session_ref_writer,
                                Some(provider_session_ref.as_str()),
                            );
                            if let Some(provider_session_ref) = updated_session {
                                persist_stage_progress(
                                    &workspace_for_ws,
                                    &conversation_for_ws,
                                    stage_index,
                                    &stage_name_for_ws,
                                    &agent_label_for_ws,
                                    &started_at_for_ws,
                                    &output_buffer_writer,
                                    Some(score_id.clone()),
                                    Some(provider_session_ref),
                                );
                            }
                            emit_pipeline_debug(
                                &app_for_ws,
                                &workspace_for_ws,
                                &conversation_for_ws,
                                format!(
                                    "{stage_name_for_ws}: provider session captured via websocket"
                                ),
                            );
                            Ok(())
                        }
                        SymphonyLiveEvent::Ignored => Ok(()),
                    },
                )
                .await;

            if let Err(error) = result {
                eprintln!("[pipeline] {stage_name_for_ws}: Symphony WebSocket error: {error}");
                emit_pipeline_debug(
                    &app_for_ws,
                    &workspace_for_ws,
                    &conversation_for_ws,
                    format!("{stage_name_for_ws}: websocket closed with error: {error}"),
                );
            }
        });
    }

    let mut stop_requested = false;
    let mut stop_requested_at: Option<Instant> = None;
    let mut poll_failure_started_at: Option<Instant> = None;
    let mut last_status: Option<String> = None;
    let terminal_snapshot: Result<SymphonyScoreSnapshot, String> = loop {
        let watched_file_exists = Path::new(file_to_watch).exists();
        if (watchers.local_stop.load(Ordering::Acquire)
            || abort.load(Ordering::Acquire)
            || watched_file_exists)
            && !stop_requested
        {
            let stop_reason = if abort.load(Ordering::Acquire) {
                "abort flag set"
            } else if watched_file_exists {
                "watched file exists"
            } else {
                "watched file became ready"
            };
            emit_pipeline_debug(
                app,
                workspace_path,
                conversation_id,
                format!(
                    "{stage_name}: issuing stop request for score {} because {stop_reason}",
                    accepted.score_id
                ),
            );
            request_symphony_stop(accepted.score_id.clone());
            stop_requested = true;
            stop_requested_at = Some(Instant::now());
        }

        match fetch_score_snapshot(&accepted.score_id).await {
            Ok(snapshot) => {
                if let Some(failure_started_at) = poll_failure_started_at.take() {
                    emit_pipeline_debug(
                        app,
                        workspace_path,
                        conversation_id,
                        format!(
                            "{stage_name}: polling recovered after {:.1}s",
                            failure_started_at.elapsed().as_secs_f32(),
                        ),
                    );
                }
                let status_label = format!("{:?}", snapshot.status).to_lowercase();
                if last_status.as_deref() != Some(status_label.as_str()) {
                    emit_pipeline_debug(
                        app,
                        workspace_path,
                        conversation_id,
                        format!(
                            "{stage_name}: polled status -> {status_label} (chars={}, final={}, error={}, exit_code={})",
                            snapshot.accumulated_text.len(),
                            snapshot.final_text.as_ref().map(|text| !text.is_empty()).unwrap_or(false),
                            snapshot.error.as_deref().unwrap_or("none"),
                            snapshot.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "none".to_string()),
                        ),
                    );
                    last_status = Some(status_label);
                }
                if let Some(delta) =
                    sync_snapshot_output(&output_buffer, &snapshot.accumulated_text)
                {
                    if let Err(error) =
                        emit_stage_delta(app, conversation_id, stage_index, &delta)
                    {
                        eprintln!("[pipeline] {stage_name}: failed to emit stage delta: {error}");
                    }
                }
                if let Some(provider_session_ref) =
                    maybe_update_session_ref(&session_ref, snapshot.provider_session_ref.as_deref())
                {
                    persist_stage_progress(
                        workspace_path,
                        conversation_id,
                        stage_index,
                        stage_name,
                        agent_label,
                        started_at,
                        &output_buffer,
                        Some(accepted.score_id.clone()),
                        Some(provider_session_ref),
                    );
                }
                if snapshot.status.is_terminal() {
                    emit_pipeline_debug(
                        app,
                        workspace_path,
                        conversation_id,
                        format!("{stage_name}: reached terminal snapshot state"),
                    );
                    break Ok(snapshot);
                }
                if stop_requested
                    && Path::new(file_to_watch).exists()
                    && stop_requested_at
                        .map(|instant| instant.elapsed() >= FILE_COMPLETION_STOP_GRACE)
                        .unwrap_or(false)
                {
                    emit_pipeline_debug(
                        app,
                        workspace_path,
                        conversation_id,
                        format!(
                            "{stage_name}: watched file exists but score {} stayed non-terminal after stop grace; finalising from file",
                            accepted.score_id,
                        ),
                    );
                    break Err(format!(
                        "{stage_name} stop grace expired while waiting for Symphony score {} to terminate",
                        accepted.score_id,
                    ));
                }
            }
            Err(error) => {
                let first_failure = poll_failure_started_at.is_none();
                let failure_started_at = poll_failure_started_at.get_or_insert_with(Instant::now);
                if Path::new(file_to_watch).exists() {
                    if !stop_requested {
                        emit_pipeline_debug(
                            app,
                            workspace_path,
                            conversation_id,
                            format!(
                                "{stage_name}: polling failed after watched file appeared; issuing best-effort stop for score {}",
                                accepted.score_id,
                            ),
                        );
                        request_symphony_stop(accepted.score_id.clone());
                    }
                    emit_pipeline_debug(
                        app,
                        workspace_path,
                        conversation_id,
                        format!(
                            "{stage_name}: polling failed after watched file existed; finalising from file: {error}",
                        ),
                    );
                    break Err(error);
                }
                if failure_started_at.elapsed() < POLL_FAILURE_GRACE {
                    if first_failure {
                        emit_pipeline_debug(
                            app,
                            workspace_path,
                            conversation_id,
                            format!(
                                "{stage_name}: polling failed but retrying for up to {}s: {error}",
                                POLL_FAILURE_GRACE.as_secs(),
                            ),
                        );
                    }
                    sleep(SCORE_POLL_INTERVAL).await;
                    continue;
                }
                emit_pipeline_debug(
                    app,
                    workspace_path,
                    conversation_id,
                    format!(
                        "{stage_name}: polling failed after {}s grace: {error}",
                        POLL_FAILURE_GRACE.as_secs(),
                    ),
                );
                break Err(error);
            }
        }

        sleep(SCORE_POLL_INTERVAL).await;
    };

    websocket_stop.store(true, Ordering::Release);

    let final_status = determine_final_status(
        abort.as_ref(),
        file_to_watch,
        watchers.file_ready.as_ref(),
        file_required,
        stage_name,
        &terminal_snapshot,
    );
    let file_text = resolve_stage_text(
        file_to_watch,
        &output_buffer,
        file_required,
        &final_status,
        stage_name,
    );
    let captured_session_ref = session_ref.lock().ok().and_then(|guard| guard.clone());
    let captured_job = score_id_slot.lock().ok().and_then(|guard| guard.clone());
    let accumulated_text = live_output(&output_buffer);
    let terminal_text = terminal_snapshot
        .as_ref()
        .ok()
        .and_then(|snapshot| snapshot.final_text.clone())
        .filter(|text| !text.trim().is_empty());
    let display_text = file_text.or(terminal_text).or_else(|| {
        if accumulated_text.is_empty() {
            None
        } else {
            Some(accumulated_text.clone())
        }
    });

    AttemptOutcome {
        terminal_snapshot,
        final_status,
        captured_session_ref,
        captured_job,
        display_text,
        accumulated_text,
    }
}
