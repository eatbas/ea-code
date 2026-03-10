/// Composite detail types used for full session/run/iteration responses
/// sent to the frontend.

use serde::{Deserialize, Serialize};

use super::records::{QuestionRow, StageRow};

/// Full session detail with all runs for the ChatView.
#[derive(Serialize, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub id: String,
    pub title: String,
    pub project_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub runs: Vec<RunDetail>,
}

/// Full run detail with iterations, stages, and questions.
#[derive(Serialize, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDetail {
    pub id: String,
    pub prompt: String,
    pub status: String,
    pub final_verdict: Option<String>,
    pub error: Option<String>,
    pub executive_summary: Option<String>,
    pub executive_summary_status: Option<String>,
    pub executive_summary_error: Option<String>,
    pub executive_summary_agent: Option<String>,
    pub executive_summary_model: Option<String>,
    pub executive_summary_generated_at: Option<String>,
    pub max_iterations: i32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub current_stage: Option<String>,
    pub current_iteration: i32,
    pub current_stage_started_at: Option<String>,
    pub iterations: Vec<IterationDetail>,
    pub questions: Vec<QuestionRow>,
}

/// Full iteration detail with stages.
#[derive(Serialize, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IterationDetail {
    pub number: i32,
    pub verdict: Option<String>,
    pub judge_reasoning: Option<String>,
    pub enhanced_prompt: Option<String>,
    pub planner_plan: Option<String>,
    pub audit_verdict: Option<String>,
    pub audit_reasoning: Option<String>,
    pub audited_plan: Option<String>,
    pub review_output: Option<String>,
    pub review_user_guidance: Option<String>,
    pub fix_output: Option<String>,
    pub judge_output: Option<String>,
    pub generate_question: Option<String>,
    pub generate_answer: Option<String>,
    pub fix_question: Option<String>,
    pub fix_answer: Option<String>,
    pub stages: Vec<StageRow>,
}
