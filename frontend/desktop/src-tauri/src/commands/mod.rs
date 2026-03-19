pub(crate) mod app;
pub(crate) mod cli;
pub(crate) mod cli_http;
pub(crate) mod cli_util;
pub(crate) mod cli_version;
#[cfg(target_os = "windows")]
pub(crate) mod git_bash;
pub(crate) mod history;
pub(crate) mod mcp;
pub(crate) mod mcp_runtime;
pub(crate) mod pipeline;
pub(crate) mod settings;
pub(crate) mod skills;
pub(crate) mod workspace;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::models::PipelineAnswer;

pub type RunCancelFlag = Arc<AtomicBool>;
pub type RunPauseFlag = Arc<AtomicBool>;
pub type PipelineAnswerSender = tokio::sync::oneshot::Sender<PipelineAnswer>;
pub type RunAnswerSender = Arc<Mutex<Option<PipelineAnswerSender>>>;

/// Shared application state, holding per-run cancel/pause flags,
/// per-run answer channels. Database pool has been removed in favour
/// of file-based storage.
pub struct AppState {
    pub cancel_flags: Arc<Mutex<HashMap<String, RunCancelFlag>>>,
    pub pause_flags: Arc<Mutex<HashMap<String, RunPauseFlag>>>,
    pub answer_senders: Arc<Mutex<HashMap<String, RunAnswerSender>>>,
}
