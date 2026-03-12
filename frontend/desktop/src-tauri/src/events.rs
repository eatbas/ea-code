use serde::Serialize;

use crate::models::{JudgeVerdict, PipelineStage, StageStatus};

pub const EVENT_PIPELINE_STARTED: &str = "pipeline:started";
pub const EVENT_PIPELINE_STAGE: &str = "pipeline:stage";
pub const EVENT_PIPELINE_LOG: &str = "pipeline:log";
pub const EVENT_PIPELINE_ARTIFACT: &str = "pipeline:artifact";
pub const EVENT_PIPELINE_COMPLETED: &str = "pipeline:completed";
pub const EVENT_PIPELINE_ERROR: &str = "pipeline:error";
pub const EVENT_PIPELINE_QUESTION: &str = "pipeline:question";

/// Emitted when the pipeline begins execution.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStartedPayload {
    pub run_id: String,
    pub session_id: String,
    pub prompt: String,
    pub workspace_path: String,
}

/// Emitted when a pipeline stage transitions status.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStagePayload {
    pub run_id: String,
    pub stage: PipelineStage,
    pub status: StageStatus,
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Emitted for each line of CLI output (stdout or stderr).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineLogPayload {
    pub run_id: String,
    pub stage: PipelineStage,
    pub line: String,
    pub stream: String,
}

/// Emitted when a notable artefact is produced (diff, review, etc.).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineArtifactPayload {
    pub run_id: String,
    pub kind: String,
    pub content: String,
    pub iteration: u32,
}

/// Emitted when the pipeline finishes successfully.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineCompletedPayload {
    pub run_id: String,
    pub verdict: JudgeVerdict,
    pub total_iterations: u32,
    pub duration_ms: u64,
}

/// Emitted when the pipeline encounters an error.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineErrorPayload {
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<PipelineStage>,
    pub message: String,
}

/// Emitted when the pipeline pauses to ask the user a question.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineQuestionPayload {
    pub run_id: String,
    pub question_id: String,
    pub stage: PipelineStage,
    pub iteration: u32,
    pub question_text: String,
    pub agent_output: String,
    pub optional: bool,
}
