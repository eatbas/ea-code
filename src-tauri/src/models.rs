use serde::{Deserialize, Serialize};

/// Agent role identifiers for the orchestration pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Generator,
    Reviewer,
    Fixer,
    Validator,
    FinalJudge,
}

/// Supported CLI agent backends.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentBackend {
    Claude,
    Codex,
    Gemini,
}

/// Pipeline stage identifiers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    Generate,
    DiffAfterGenerate,
    Review,
    Fix,
    DiffAfterFix,
    Validate,
    Judge,
}

/// Status of a single pipeline stage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
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
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Idle,
    Running,
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

/// Request to start a pipeline run.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineRequest {
    pub prompt: String,
    pub workspace_path: String,
}

/// Workspace validation result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub path: String,
    pub is_git_repo: bool,
    pub is_dirty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// CLI health check result per binary.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub available: bool,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregate CLI health check result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliHealth {
    pub claude: CliStatus,
    pub codex: CliStatus,
    pub gemini: CliStatus,
}

/// Application settings persisted locally.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub claude_path: String,
    pub codex_path: String,
    pub gemini_path: String,
    pub generator_agent: AgentBackend,
    pub reviewer_agent: AgentBackend,
    pub fixer_agent: AgentBackend,
    pub validator_agent: AgentBackend,
    pub final_judge_agent: AgentBackend,
    pub max_iterations: u32,
    pub require_git: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            codex_path: "codex".to_string(),
            gemini_path: "gemini".to_string(),
            generator_agent: AgentBackend::Claude,
            reviewer_agent: AgentBackend::Codex,
            fixer_agent: AgentBackend::Claude,
            validator_agent: AgentBackend::Gemini,
            final_judge_agent: AgentBackend::Codex,
            max_iterations: 3,
            require_git: true,
        }
    }
}
