use futures::StreamExt;
use serde::Deserialize;

use crate::models::ConversationStatus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HiveSseEvent {
    RunStarted { job_id: String },
    ProviderSession { provider_session_ref: String },
    OutputDelta { text: String },
    Completed {
        final_text: String,
        provider_session_ref: Option<String>,
        exit_code: i32,
    },
    Failed {
        error: String,
        provider_session_ref: Option<String>,
        exit_code: Option<i32>,
    },
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseResult {
    pub final_text: String,
    pub provider_session_ref: Option<String>,
    pub exit_code: Option<i32>,
    pub status: ConversationStatus,
    pub error: Option<String>,
    pub job_id: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RunStartedPayload {
    job_id: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ProviderSessionPayload {
    provider_session_ref: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OutputDeltaPayload {
    text: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CompletedPayload {
    final_text: String,
    provider_session_ref: Option<String>,
    exit_code: i32,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct FailedPayload {
    error: String,
    provider_session_ref: Option<String>,
    exit_code: Option<i32>,
}

pub async fn consume_hive_sse<F>(
    response: reqwest::Response,
    mut on_event: F,
) -> Result<SseResult, String>
where
    F: FnMut(&HiveSseEvent) -> Result<(), String>,
{
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut collected_text = String::new();
    let mut session_ref: Option<String> = None;
    let mut job_id: Option<String> = None;
    let mut terminal_result: Option<SseResult> = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|error| format!("Failed to read hive stream: {error}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(frame_end) = buffer.find("\n\n") {
            let frame = buffer[..frame_end].to_string();
            buffer.drain(..frame_end + 2);

            let Some(event) = parse_frame(&frame)? else {
                continue;
            };

            match &event {
                HiveSseEvent::RunStarted { job_id: current_job_id } => {
                    job_id = Some(current_job_id.clone());
                }
                HiveSseEvent::ProviderSession { provider_session_ref } => {
                    session_ref = Some(provider_session_ref.clone());
                }
                HiveSseEvent::OutputDelta { text } => {
                    if !collected_text.is_empty() {
                        collected_text.push('\n');
                    }
                    collected_text.push_str(text);
                }
                HiveSseEvent::Completed {
                    final_text,
                    provider_session_ref,
                    exit_code,
                } => {
                    let resolved_text = if final_text.trim().is_empty() {
                        collected_text.clone()
                    } else {
                        final_text.clone()
                    };
                    let resolved_session = provider_session_ref.clone().or_else(|| session_ref.clone());
                    terminal_result = Some(SseResult {
                        final_text: resolved_text,
                        provider_session_ref: resolved_session,
                        exit_code: Some(*exit_code),
                        status: ConversationStatus::Completed,
                        error: None,
                        job_id: job_id.clone(),
                    });
                }
                HiveSseEvent::Failed {
                    error,
                    provider_session_ref,
                    exit_code,
                } => {
                    let resolved_session = provider_session_ref.clone().or_else(|| session_ref.clone());
                    terminal_result = Some(SseResult {
                        final_text: collected_text.clone(),
                        provider_session_ref: resolved_session,
                        exit_code: *exit_code,
                        status: ConversationStatus::Failed,
                        error: Some(error.clone()),
                        job_id: job_id.clone(),
                    });
                }
                HiveSseEvent::Stopped => {
                    terminal_result = Some(SseResult {
                        final_text: collected_text.clone(),
                        provider_session_ref: session_ref.clone(),
                        exit_code: None,
                        status: ConversationStatus::Stopped,
                        error: None,
                        job_id: job_id.clone(),
                    });
                }
            }

            on_event(&event)?;
            if terminal_result.is_some() {
                return terminal_result.ok_or_else(|| "Missing terminal hive stream state".to_string());
            }
        }
    }

    Err("Hive stream ended before a terminal event was received".to_string())
}

fn parse_frame(frame: &str) -> Result<Option<HiveSseEvent>, String> {
    let mut event_name: Option<&str> = None;
    let mut data_lines: Vec<&str> = Vec::new();

    for line in frame.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            event_name = Some(rest.trim());
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            data_lines.push(rest.trim_start());
        }
    }

    let Some(event_name) = event_name else {
        return Ok(None);
    };
    let payload = data_lines.join("\n");

    match event_name {
        "run_started" => Ok(Some(HiveSseEvent::RunStarted {
            job_id: serde_json::from_str::<RunStartedPayload>(&payload)
                .map_err(|error| format!("Failed to parse run_started payload: {error}"))?
                .job_id,
        })),
        "provider_session" => Ok(Some(HiveSseEvent::ProviderSession {
            provider_session_ref: serde_json::from_str::<ProviderSessionPayload>(&payload)
                .map_err(|error| format!("Failed to parse provider_session payload: {error}"))?
                .provider_session_ref,
        })),
        "output_delta" => Ok(Some(HiveSseEvent::OutputDelta {
            text: serde_json::from_str::<OutputDeltaPayload>(&payload)
                .map_err(|error| format!("Failed to parse output_delta payload: {error}"))?
                .text,
        })),
        "completed" => {
            let parsed = serde_json::from_str::<CompletedPayload>(&payload)
                .map_err(|error| format!("Failed to parse completed payload: {error}"))?;
            Ok(Some(HiveSseEvent::Completed {
                final_text: parsed.final_text,
                provider_session_ref: parsed.provider_session_ref,
                exit_code: parsed.exit_code,
            }))
        }
        "failed" => {
            let parsed = serde_json::from_str::<FailedPayload>(&payload)
                .map_err(|error| format!("Failed to parse failed payload: {error}"))?;
            Ok(Some(HiveSseEvent::Failed {
                error: parsed.error,
                provider_session_ref: parsed.provider_session_ref,
                exit_code: parsed.exit_code,
            }))
        }
        "stopped" => Ok(Some(HiveSseEvent::Stopped)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_frame, HiveSseEvent};

    #[test]
    fn parses_output_delta_frame() {
        let frame = "event: output_delta\ndata: {\"text\":\"hello\"}";
        let parsed = parse_frame(frame).expect("frame should parse");
        assert_eq!(
            parsed,
            Some(HiveSseEvent::OutputDelta {
                text: "hello".to_string()
            })
        );
    }

    #[test]
    fn parses_completed_frame() {
        let frame = "event: completed\ndata: {\"final_text\":\"done\",\"provider_session_ref\":\"abc\",\"exit_code\":0}";
        let parsed = parse_frame(frame).expect("frame should parse");
        assert_eq!(
            parsed,
            Some(HiveSseEvent::Completed {
                final_text: "done".to_string(),
                provider_session_ref: Some("abc".to_string()),
                exit_code: 0,
            })
        );
    }
}
