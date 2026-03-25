use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::agents::base::{AgentInput, AgentOutput};
use crate::events::{PipelineLogPayload, EVENT_PIPELINE_LOG};
use crate::models::PipelineStage;

/// Request body matching hive-api's `POST /v1/chat` schema.
#[derive(Serialize)]
pub struct ChatRequest {
    pub provider: String,
    pub model: String,
    pub workspace_path: String,
    pub mode: String,
    pub prompt: String,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_session_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<serde_json::Value>,
}

/// SSE `completed` event data from hive-api.
#[derive(Deserialize)]
struct CompletedData {
    final_text: String,
    #[allow(dead_code)]
    exit_code: i32,
    provider_session_ref: Option<String>,
}

/// SSE `provider_session` event data from hive-api.
#[derive(Deserialize)]
struct ProviderSessionData {
    provider_session_ref: String,
}

/// SSE `failed` event data from hive-api.
#[derive(Deserialize)]
struct FailedData {
    error: String,
    #[allow(dead_code)]
    exit_code: Option<i32>,
}

/// SSE `run_started` event data from hive-api.
#[derive(Deserialize)]
struct RunStartedData {
    job_id: String,
}

/// SSE `output_delta` event data from hive-api.
#[derive(Deserialize)]
struct OutputDeltaData {
    text: String,
}

/// Result of a successful API agent call.
pub struct ApiAgentResult {
    pub output: AgentOutput,
    pub job_id: String,
    pub provider_session_ref: Option<String>,
}

/// Send a prompt to hive-api via SSE streaming, emitting `pipeline:log`
/// events for each output chunk, and return the final aggregated text.
///
/// When `session_ref` is provided, the request uses `mode: "resume"` to
/// continue an existing CLI session instead of starting a fresh one.
///
/// When `abort_flag` is provided and set to `true` during the SSE read loop,
/// the function cancels the running job (if a `job_id` was received) and
/// returns the text accumulated so far. This enables the "output file
/// watchdog" to short-circuit agents that write their report to disk but
/// fail to exit cleanly.
pub async fn run_api_agent(
    base_url: &str,
    input: &AgentInput,
    provider: &str,
    model: &str,
    app: &AppHandle,
    run_id: &str,
    stage: PipelineStage,
    session_ref: Option<&str>,
    abort_flag: Option<Arc<AtomicBool>>,
) -> Result<ApiAgentResult, String> {
    let full_prompt = crate::agents::base::build_full_prompt(input);

    let mode = if session_ref.is_some() { "resume" } else { "new" };
    let request = ChatRequest {
        provider: provider.to_string(),
        model: model.to_string(),
        workspace_path: input.workspace_path.clone(),
        mode: mode.to_string(),
        prompt: full_prompt,
        stream: true,
        provider_session_ref: session_ref.map(|s| s.to_string()),
        provider_options: None,
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{base_url}/v1/chat");
    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to hive-api: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("hive-api returned {status}: {body}"));
    }

    // Parse SSE stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut accumulated_text = String::new();
    let mut job_id = String::new();
    let mut session_ref_out: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        // Check abort flag before processing each chunk.
        if let Some(ref flag) = abort_flag {
            if flag.load(Ordering::Relaxed) {
                eprintln!("[info] abort_flag set — cancelling job {job_id} and returning accumulated text");
                if !job_id.is_empty() {
                    let _ = cancel_api_job(base_url, &job_id).await;
                }
                return Ok(ApiAgentResult {
                    output: AgentOutput {
                        raw_text: accumulated_text,
                    },
                    job_id,
                    provider_session_ref: session_ref_out,
                });
            }
        }

        let chunk = chunk.map_err(|e| format!("SSE stream error: {e}"))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // Process complete SSE messages (separated by double newline)
        while let Some(pos) = buffer.find("\n\n") {
            let message = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            if let Some((event_name, data)) = parse_sse_message(&message) {
                match event_name.as_str() {
                    "run_started" => {
                        if let Ok(started) =
                            serde_json::from_str::<RunStartedData>(&data)
                        {
                            job_id = started.job_id;
                        }
                    }
                    "output_delta" => {
                        if let Ok(delta) =
                            serde_json::from_str::<OutputDeltaData>(&data)
                        {
                            accumulated_text.push_str(&delta.text);
                            // Emit each delta as a pipeline:log event
                            for line in delta.text.lines() {
                                let _ = app.emit(
                                    EVENT_PIPELINE_LOG,
                                    PipelineLogPayload {
                                        run_id: run_id.to_string(),
                                        stage: stage.clone(),
                                        line: line.to_string(),
                                        stream: "stdout".to_string(),
                                    },
                                );
                            }
                        }
                    }
                    "provider_session" => {
                        if let Ok(ps) =
                            serde_json::from_str::<ProviderSessionData>(&data)
                        {
                            session_ref_out = Some(ps.provider_session_ref);
                        }
                    }
                    "completed" => {
                        if let Ok(completed) =
                            serde_json::from_str::<CompletedData>(&data)
                        {
                            // Prefer session ref from completed event; fall back to earlier capture.
                            let final_ref = completed
                                .provider_session_ref
                                .or(session_ref_out);
                            return Ok(ApiAgentResult {
                                output: AgentOutput {
                                    raw_text: completed.final_text,
                                },
                                job_id,
                                provider_session_ref: final_ref,
                            });
                        }
                    }
                    "failed" => {
                        let error = serde_json::from_str::<FailedData>(&data)
                            .map(|f| f.error)
                            .unwrap_or_else(|_| data.clone());
                        return Err(format!("Agent failed: {error}"));
                    }
                    "stopped" => {
                        return Err("Agent was stopped/cancelled".into());
                    }
                    _ => {
                        // Unknown event types — skip silently.
                    }
                }
            }
        }
    }

    // Stream ended without a terminal event — return accumulated text
    if !accumulated_text.is_empty() {
        Ok(ApiAgentResult {
            output: AgentOutput {
                raw_text: accumulated_text,
            },
            job_id,
            provider_session_ref: session_ref_out,
        })
    } else {
        Err("SSE stream ended without completing".into())
    }
}

/// Cancel a running job by posting to `/v1/chat/{job_id}/stop`.
pub async fn cancel_api_job(base_url: &str, job_id: &str) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{base_url}/v1/chat/{job_id}/stop");
    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to cancel job: {e}"))?;

    if response.status().is_success() || response.status().as_u16() == 404 {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(format!("Cancel failed: {body}"))
    }
}

/// Parse a single SSE message into (event_name, data_json).
fn parse_sse_message(message: &str) -> Option<(String, String)> {
    let mut event_name = String::new();
    let mut data_lines = Vec::new();

    for line in message.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_name = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data: ") {
            data_lines.push(value);
        } else if line.starts_with("event:") {
            event_name = line[6..].trim().to_string();
        } else if line.starts_with("data:") {
            data_lines.push(line[5..].trim());
        }
    }

    if event_name.is_empty() || data_lines.is_empty() {
        return None;
    }

    Some((event_name, data_lines.join("\n")))
}
