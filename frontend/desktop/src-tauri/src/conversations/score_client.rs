use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::StreamExt;
use serde::Deserialize;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::commands::api_health::symphony_base_url;
use crate::http::symphony_client;
use crate::models::ConversationStatus;

use super::symphony_request::SymphonyChatRequest;

pub const SCORE_POLL_INTERVAL: Duration = Duration::from_secs(1);
const SCORE_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymphonyScoreStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Stopped,
}

impl SymphonyScoreStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Stopped)
    }

    pub fn as_conversation_status(&self) -> ConversationStatus {
        match self {
            Self::Completed => ConversationStatus::Completed,
            Self::Failed => ConversationStatus::Failed,
            Self::Stopped => ConversationStatus::Stopped,
            Self::Queued | Self::Running => ConversationStatus::Running,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SymphonyScoreAcceptedResponse {
    pub score_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SymphonyScoreSnapshot {
    pub status: SymphonyScoreStatus,
    pub accumulated_text: String,
    pub final_text: Option<String>,
    pub provider_session_ref: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug)]
pub enum SymphonyLiveEvent {
    ScoreSnapshot(SymphonyScoreSnapshot),
    OutputDelta { text: String },
    ProviderSession { provider_session_ref: String },
    Ignored,
}

#[derive(Deserialize)]
struct ScoreSnapshotEnvelope {
    score: SymphonyScoreSnapshot,
}

pub async fn submit_score(
    request: &SymphonyChatRequest<'_>,
) -> Result<SymphonyScoreAcceptedResponse, String> {
    let url = format!("{}/v1/chat", symphony_base_url());
    let response = symphony_client()
        .post(&url)
        .json(request)
        .timeout(SCORE_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| format!("Failed to submit Symphony score: {error}"))?;

    if response.status().as_u16() != 202 {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Symphony submit failed: HTTP {status}: {body}"));
    }

    response
        .json::<SymphonyScoreAcceptedResponse>()
        .await
        .map_err(|error| format!("Failed to parse Symphony submit response: {error}"))
}

pub async fn fetch_score_snapshot(score_id: &str) -> Result<SymphonyScoreSnapshot, String> {
    let url = format!("{}/v1/chat/{score_id}", symphony_base_url());
    let response = symphony_client()
        .get(&url)
        .timeout(SCORE_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| format!("Failed to fetch Symphony score {score_id}: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Failed to fetch Symphony score {score_id}: HTTP {status}: {body}"
        ));
    }

    response
        .json::<SymphonyScoreSnapshot>()
        .await
        .map_err(|error| format!("Failed to parse Symphony score {score_id}: {error}"))
}

pub async fn poll_until_terminal<F>(
    score_id: &str,
    stop_flag: &AtomicBool,
    mut on_snapshot: F,
) -> Result<SymphonyScoreSnapshot, String>
where
    F: FnMut(&SymphonyScoreSnapshot) -> Result<(), String>,
{
    loop {
        let snapshot = fetch_score_snapshot(score_id).await?;
        on_snapshot(&snapshot)?;
        if snapshot.status.is_terminal() {
            return Ok(snapshot);
        }
        if stop_flag.load(Ordering::Acquire) {
            sleep(SCORE_POLL_INTERVAL).await;
            continue;
        }
        sleep(SCORE_POLL_INTERVAL).await;
    }
}

pub async fn consume_score_websocket<F>(
    score_id: &str,
    stop_flag: &AtomicBool,
    mut on_event: F,
) -> Result<(), String>
where
    F: FnMut(SymphonyLiveEvent) -> Result<(), String>,
{
    let mut url = symphony_base_url();
    if let Some(rest) = url.strip_prefix("http://") {
        url = format!("ws://{rest}");
    } else if let Some(rest) = url.strip_prefix("https://") {
        url = format!("wss://{rest}");
    }
    url = format!("{url}/v1/chat/{score_id}/ws");

    let (mut socket, _) = connect_async(&url)
        .await
        .map_err(|error| format!("Failed to connect Symphony WebSocket: {error}"))?;

    loop {
        let message = tokio::select! {
            _ = wait_for_stop(stop_flag) => {
                let _ = socket.close(None).await;
                return Ok(());
            }
            next = socket.next() => next,
        };

        let Some(message) = message else {
            return Ok(());
        };

        let message = message.map_err(|error| format!("Symphony WebSocket read failed: {error}"))?;
        match message {
            Message::Text(text) => on_event(parse_live_event(&text)?)?,
            Message::Close(_) => return Ok(()),
            _ => {}
        }
    }
}

async fn wait_for_stop(stop_flag: &AtomicBool) {
    while !stop_flag.load(Ordering::Acquire) {
        sleep(Duration::from_millis(100)).await;
    }
}

fn parse_live_event(payload: &str) -> Result<SymphonyLiveEvent, String> {
    let value: serde_json::Value =
        serde_json::from_str(payload).map_err(|error| format!("Invalid Symphony WebSocket payload: {error}"))?;
    let event_type = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    match event_type {
        "score_snapshot" => serde_json::from_value::<ScoreSnapshotEnvelope>(value)
            .map(|envelope| SymphonyLiveEvent::ScoreSnapshot(envelope.score))
            .map_err(|error| format!("Invalid Symphony score snapshot payload: {error}")),
        "output_delta" => Ok(SymphonyLiveEvent::OutputDelta {
            text: value
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "provider_session" => Ok(SymphonyLiveEvent::ProviderSession {
            provider_session_ref: value
                .get("provider_session_ref")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        _ => Ok(SymphonyLiveEvent::Ignored),
    }
}
