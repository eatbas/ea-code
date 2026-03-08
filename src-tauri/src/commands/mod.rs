mod cli;
mod history;
mod pipeline;
mod settings;
mod workspace;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::models::PipelineAnswer;

// Re-export all command functions for use in lib.rs invoke_handler
pub use cli::{check_cli_health, get_cli_versions, update_cli};
pub use history::{
    create_session, delete_session, get_run_artifacts, get_run_detail, get_run_logs,
    get_session_detail, list_projects, list_sessions,
};
pub use pipeline::{answer_pipeline_question, cancel_pipeline, run_pipeline};
pub use settings::{get_settings, save_settings};
pub use workspace::{select_workspace, validate_environment};

/// Shared application state, holding the pipeline cancellation flag,
/// the oneshot channel for delivering user answers, and the database pool.
pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub answer_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<PipelineAnswer>>>>,
    pub db: DbPool,
}
