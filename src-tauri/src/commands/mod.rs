pub(crate) mod cli;
pub(crate) mod history;
pub(crate) mod mcp;
pub(crate) mod pipeline;
pub(crate) mod settings;
pub(crate) mod skills;
pub(crate) mod workspace;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::models::PipelineAnswer;

/// Shared application state, holding the pipeline cancellation flag,
/// the oneshot channel for delivering user answers, and the database pool.
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub answer_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    pub db: DbPool,
}
