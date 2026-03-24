use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// Execution intent for a pipeline stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageExecutionIntent {
    Text,
    Code,
}

/// Pipeline stage identifiers.
#[derive(Clone, Debug, PartialEq)]
pub enum PipelineStage {
    PromptEnhance,
    SkillSelect,
    Plan,
    /// Extra planner slot (0-indexed: ExtraPlan(0) = planner 2, ExtraPlan(1) = planner 3, etc.)
    ExtraPlan(u8),
    PlanAudit,
    Coder,
    CodeReviewer,
    /// Extra reviewer slot (0-indexed: ExtraReviewer(0) = reviewer 2, etc.)
    ExtraReviewer(u8),
    /// Review Merger — combines findings from multiple reviewers.
    ReviewMerge,
    CodeFixer,
    Judge,
    ExecutiveSummary,
    DirectTask,
}

impl Serialize for PipelineStage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            PipelineStage::PromptEnhance => serializer.serialize_str("prompt_enhance"),
            PipelineStage::SkillSelect => serializer.serialize_str("skill_select"),
            PipelineStage::Plan => serializer.serialize_str("plan"),
            PipelineStage::ExtraPlan(i) => serializer.serialize_str(&format!("plan{}", i + 2)),
            PipelineStage::PlanAudit => serializer.serialize_str("plan_audit"),
            PipelineStage::Coder => serializer.serialize_str("coder"),
            PipelineStage::CodeReviewer => serializer.serialize_str("code_reviewer"),
            PipelineStage::ExtraReviewer(i) => {
                serializer.serialize_str(&format!("code_reviewer{}", i + 2))
            }
            PipelineStage::ReviewMerge => serializer.serialize_str("review_merge"),
            PipelineStage::CodeFixer => serializer.serialize_str("code_fixer"),
            PipelineStage::Judge => serializer.serialize_str("judge"),
            PipelineStage::ExecutiveSummary => serializer.serialize_str("executive_summary"),
            PipelineStage::DirectTask => serializer.serialize_str("direct_task"),
        }
    }
}

impl<'de> Deserialize<'de> for PipelineStage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "prompt_enhance" => Ok(PipelineStage::PromptEnhance),
            "skill_select" => Ok(PipelineStage::SkillSelect),
            "plan" => Ok(PipelineStage::Plan),
            "plan_audit" => Ok(PipelineStage::PlanAudit),
            "coder" => Ok(PipelineStage::Coder),
            "code_reviewer" => Ok(PipelineStage::CodeReviewer),
            "review_merge" => Ok(PipelineStage::ReviewMerge),
            "code_fixer" => Ok(PipelineStage::CodeFixer),
            "judge" => Ok(PipelineStage::Judge),
            "executive_summary" => Ok(PipelineStage::ExecutiveSummary),
            "direct_task" => Ok(PipelineStage::DirectTask),
            other => {
                // Dynamic planner: plan2, plan3, plan4, ...
                if let Some(suffix) = other.strip_prefix("plan") {
                    if let Ok(n) = suffix.parse::<u8>() {
                        if n >= 2 {
                            return Ok(PipelineStage::ExtraPlan(n - 2));
                        }
                    }
                }
                // Dynamic reviewer: code_reviewer2, code_reviewer3, ...
                if let Some(suffix) = other.strip_prefix("code_reviewer") {
                    if let Ok(n) = suffix.parse::<u8>() {
                        if n >= 2 {
                            return Ok(PipelineStage::ExtraReviewer(n - 2));
                        }
                    }
                }
                Err(serde::de::Error::unknown_variant(
                    other,
                    &[
                        "prompt_enhance",
                        "skill_select",
                        "plan",
                        "plan2",
                        "plan3",
                        "plan_audit",
                        "coder",
                        "code_reviewer",
                        "code_reviewer2",
                        "code_reviewer3",
                        "review_merge",
                        "code_fixer",
                        "judge",
                        "executive_summary",
                        "direct_task",
                    ],
                ))
            }
        }
    }
}

impl PipelineStage {
    pub fn execution_intent(&self) -> StageExecutionIntent {
        match self {
            PipelineStage::Coder | PipelineStage::CodeFixer | PipelineStage::DirectTask => {
                StageExecutionIntent::Code
            }
            PipelineStage::PromptEnhance
            | PipelineStage::SkillSelect
            | PipelineStage::Plan
            | PipelineStage::ExtraPlan(_)
            | PipelineStage::PlanAudit
            | PipelineStage::CodeReviewer
            | PipelineStage::ExtraReviewer(_)
            | PipelineStage::ReviewMerge
            | PipelineStage::Judge
            | PipelineStage::ExecutiveSummary => StageExecutionIntent::Text,
        }
    }

    pub fn requires_output_file(&self) -> bool {
        matches!(self.execution_intent(), StageExecutionIntent::Text)
    }
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
    Paused,
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
    /// CLI session reference returned by hive-api for session continuity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_session_ref: Option<String>,
    /// Which session pair this stage belongs to (e.g. "plan_review", "code_fix").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_pair: Option<String>,
    /// Whether this stage resumed an existing CLI session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resumed: Option<bool>,
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
