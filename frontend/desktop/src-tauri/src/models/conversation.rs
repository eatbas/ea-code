use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSelection {
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMessageRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    pub id: String,
    pub role: ConversationMessageRole,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub workspace_path: String,
    pub agent: AgentSelection,
    pub status: ConversationStatus,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub last_provider_session_ref: Option<String>,
    pub active_job_id: Option<String>,
    pub error: Option<String>,
    pub archived_at: Option<String>,
    pub pinned_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationDetail {
    pub summary: ConversationSummary,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationOutputDelta {
    pub conversation_id: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationStatusEvent {
    pub conversation: ConversationSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<ConversationMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageStatusEvent {
    pub conversation_id: String,
    pub stage_index: usize,
    pub stage_name: String,
    pub status: ConversationStatus,
    pub agent_label: String,
    /// When a stage completes, this carries the authoritative plan file
    /// content so the frontend can replace the accumulated SSE output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageOutputDelta {
    pub conversation_id: String,
    pub stage_index: usize,
    pub text: String,
}

/// Persisted state of a single pipeline stage.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStageRecord {
    pub stage_index: usize,
    pub stage_name: String,
    pub agent_label: String,
    pub status: ConversationStatus,
    pub text: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    /// Hive-API job ID for this stage's run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    /// Provider session ref for resuming this stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_session_ref: Option<String>,
}

/// Persisted pipeline state saved alongside conversation.json.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineState {
    pub user_prompt: String,
    pub pipeline_mode: String,
    pub stages: Vec<PipelineStageRecord>,
}
