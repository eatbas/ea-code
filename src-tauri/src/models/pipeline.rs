use serde::{Deserialize, Serialize};

/// Pipeline stage identifiers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    PromptEnhance,
    Plan,
    PlanAudit,
    Generate,
    DiffAfterGenerate,
    Review,
    Fix,
    DiffAfterFix,
    Judge,
    ExecutiveSummary,
}

/// Status of a single pipeline stage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    WaitingForInput,
}

/// Judge verdict — the final arbiter's decision.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JudgeVerdict {
    #[serde(rename = "COMPLETE")]
    Complete,
    #[serde(rename = "NOT COMPLETE")]
    NotComplete,
}

/// Overall pipeline run status.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Idle,
    Running,
    WaitingForInput,
    Completed,
    Failed,
    Cancelled,
}

/// Represents one stage's result in the pipeline timeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageResult {
    pub stage: PipelineStage,
    pub status: StageStatus,
    pub output: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A single iteration of the self-improving loop.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Iteration {
    pub number: u32,
    pub stages: Vec<StageResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<JudgeVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judge_reasoning: Option<String>,
}

/// Full pipeline run state for the frontend.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRun {
    pub id: String,
    pub status: PipelineStatus,
    pub prompt: String,
    pub workspace_path: String,
    pub iterations: Vec<Iteration>,
    pub current_iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<PipelineStage>,
    pub max_iterations: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_verdict: Option<JudgeVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
