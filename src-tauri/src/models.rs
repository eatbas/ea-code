use serde::{Deserialize, Serialize};

/// Agent role identifiers for the orchestration pipeline.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    PromptEnhancer,
    Planner,
    PlanAuditor,
    Coder,
    ReviewerAuditor,
    CodeFixer,
    Judge,
}

/// Supported CLI agent backends.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentBackend {
    Claude,
    Codex,
    Gemini,
}

fn default_prompt_enhancer_agent() -> AgentBackend {
    AgentBackend::Claude
}

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

/// Version and availability information for a single CLI tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliVersionInfo {
    pub name: String,
    pub cli_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub up_to_date: bool,
    pub update_command: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregate version information for all CLI tools.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllCliVersions {
    pub claude: CliVersionInfo,
    pub codex: CliVersionInfo,
    pub gemini: CliVersionInfo,
}

/// Application settings persisted locally.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub claude_path: String,
    pub codex_path: String,
    pub gemini_path: String,
    #[serde(default = "default_prompt_enhancer_agent")]
    pub prompt_enhancer_agent: AgentBackend,
    #[serde(default)]
    pub planner_agent: Option<AgentBackend>,
    #[serde(default)]
    pub plan_auditor_agent: Option<AgentBackend>,
    pub generator_agent: AgentBackend,
    pub reviewer_agent: AgentBackend,
    pub fixer_agent: AgentBackend,
    pub final_judge_agent: AgentBackend,
    pub max_iterations: u32,
    pub require_git: bool,
    /// Comma-separated list of enabled Claude models.
    pub claude_model: String,
    /// Comma-separated list of enabled Codex models.
    pub codex_model: String,
    /// Comma-separated list of enabled Gemini models.
    pub gemini_model: String,
    /// Per-stage model selections.
    pub prompt_enhancer_model: String,
    #[serde(default)]
    pub planner_model: Option<String>,
    #[serde(default)]
    pub plan_auditor_model: Option<String>,
    pub generator_model: String,
    pub reviewer_model: String,
    pub fixer_model: String,
    pub final_judge_model: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            codex_path: "codex".to_string(),
            gemini_path: "gemini".to_string(),
            prompt_enhancer_agent: AgentBackend::Claude,
            planner_agent: None,
            plan_auditor_agent: None,
            generator_agent: AgentBackend::Claude,
            reviewer_agent: AgentBackend::Codex,
            fixer_agent: AgentBackend::Claude,
            final_judge_agent: AgentBackend::Codex,
            max_iterations: 3,
            require_git: true,
            claude_model: "sonnet".to_string(),
            codex_model: "codex-5.3".to_string(),
            gemini_model: "gemini-2.5-pro".to_string(),
            prompt_enhancer_model: "sonnet".to_string(),
            planner_model: None,
            plan_auditor_model: None,
            generator_model: "sonnet".to_string(),
            reviewer_model: "codex-5.3".to_string(),
            fixer_model: "sonnet".to_string(),
            final_judge_model: "codex-5.3".to_string(),
        }
    }
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
