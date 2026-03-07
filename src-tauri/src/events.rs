use serde::Serialize;

use crate::models::{JudgeVerdict, PipelineStage, StageStatus};

/// Emitted when the pipeline begins execution.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStartedPayload {
    pub run_id: String,
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
