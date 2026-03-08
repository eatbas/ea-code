use serde::{Deserialize, Serialize};

use super::pipeline::PipelineStage;

/// Request to start a pipeline run.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRequest {
    pub prompt: String,
    pub workspace_path: String,
    /// Session ID for this conversation thread.
    /// If not provided, a new session will be created.
    pub session_id: Option<String>,
}

/// Represents a question posed by the pipeline to the user between stages.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineQuestion {
    pub run_id: String,
    pub question_id: String,
    pub stage: PipelineStage,
    pub iteration: u32,
    pub question_text: String,
    pub agent_output: String,
    /// Whether answering is optional (user can skip).
    pub optional: bool,
}

/// Request payload sent from the frontend to answer a pipeline question.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineAnswer {
    pub question_id: String,
    pub answer: String,
    /// If true, the user chose to skip without providing guidance.
    pub skipped: bool,
}
