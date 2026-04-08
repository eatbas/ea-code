use std::collections::VecDeque;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::storage::now_rfc3339;

const MAX_ENTRIES: usize = 2_000;
const EVENT_SIDECAR_LOG: &str = "sidecar_log";

/// A single captured line from the sidecar process.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarLogEntry {
    pub stream: String,
    pub line: String,
    pub timestamp: String,
}

/// Thread-safe, bounded ring buffer for sidecar log lines.
#[derive(Clone)]
pub struct SidecarLogBuffer {
    inner: Arc<Mutex<VecDeque<SidecarLogEntry>>>,
}

impl SidecarLogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_ENTRIES))),
        }
    }

    /// Append an entry, dropping the oldest if the buffer is full.
    pub async fn push(&self, entry: SidecarLogEntry) {
        let mut buf = self.inner.lock().await;
        if buf.len() >= MAX_ENTRIES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    /// Clone all buffered entries for the frontend.
    pub async fn snapshot(&self) -> Vec<SidecarLogEntry> {
        self.inner.lock().await.iter().cloned().collect()
    }
}

/// Timestamp, buffer, and emit a single sidecar log line.
pub fn emit_sidecar_log(
    app: Option<&AppHandle>,
    buffer: Option<&SidecarLogBuffer>,
    stream: &str,
    line: String,
) {
    let entry = SidecarLogEntry {
        stream: stream.to_string(),
        line,
        timestamp: now_rfc3339(),
    };

    if let Some(buf) = buffer {
        let buf = buf.clone();
        let entry_clone = entry.clone();
        tokio::spawn(async move {
            buf.push(entry_clone).await;
        });
    }

    if let Some(app) = app {
        let _ = app.emit(EVENT_SIDECAR_LOG, &entry);
    }
}
