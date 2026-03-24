use serde::{Deserialize, Serialize};

use super::agents::AgentBackend;
use super::events::RunEvent;
use super::pipeline::{JudgeVerdict, PipelineStage, PipelineStatus};

/// Run status for storage (includes both active and terminal states).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunFileStatus {
    Running,
    Paused,
    WaitingForInput,
    Completed,
    Failed,
    Cancelled,
    Crashed,
}

impl From<PipelineStatus> for RunFileStatus {
    fn from(status: PipelineStatus) -> Self {
        match status {
            PipelineStatus::Running => RunFileStatus::Running,
            PipelineStatus::Paused => RunFileStatus::Paused,
            PipelineStatus::WaitingForInput => RunFileStatus::WaitingForInput,
            PipelineStatus::Completed => RunFileStatus::Completed,
            PipelineStatus::Failed => RunFileStatus::Failed,
            PipelineStatus::Cancelled => RunFileStatus::Cancelled,
            PipelineStatus::Idle => RunFileStatus::Completed,
        }
    }
}

/// Project entry in the projects.json array.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub is_git_repo: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Skill file (skills/<id>.json) - individual skill definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP server configuration entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub command: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty", default)]
    pub env: std::collections::HashMap<String, String>,
}

/// MCP configuration file (mcp.json) - servers and CLI bindings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfigFile {
    pub schema_version: u32,
    #[serde(default)]
    pub servers: std::collections::HashMap<String, McpServerConfig>,
    /// CLI bindings map: CLI name -> list of MCP server IDs.
    #[serde(default)]
    pub cli_bindings: std::collections::HashMap<String, Vec<String>>,
}

/// Session metadata (projects/<pid>/sessions/<id>/session.json) - rich metadata for fast reads.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub project_id: String,
    pub project_path: String,
    #[serde(default)]
    pub run_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verdict: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Git baseline captured at run start for change detection.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBaseline {
    pub commit_sha: String,
    pub had_unstaged_changes: bool,
}

/// Run summary (sessions/<id>/runs/<rid>/summary.json) - fast read for history.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub schema_version: u32,
    pub id: String,
    pub session_id: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_prompt: Option<String>,
    pub status: RunFileStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_verdict: Option<JudgeVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<PipelineStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_iteration: Option<u32>,
    #[serde(default)]
    pub total_iterations: u32,
    pub max_iterations: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executive_summary: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub files_changed: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_baseline: Option<GitBaseline>,
    /// Path to the workspace/project directory where git commands should be executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<String>,
    /// Next sequence number for events (avoids reading entire events.jsonl file).
    #[serde(default = "default_next_sequence")]
    pub next_sequence: u64,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

fn default_next_sequence() -> u64 {
    1
}

/// Compact structured review findings for Judge (extracted from Reviewer output).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFindings {
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub nits: Vec<String>,
    #[serde(default)]
    pub tests_run: bool,
    #[serde(default)]
    pub test_commands: Vec<String>,
    #[serde(default)]
    pub test_results: Vec<String>,
    #[serde(default)]
    pub test_gaps: Vec<String>,
    /// Reviewer verdict: "PASS" or "FAIL".
    pub verdict: String,
}

/// Session detail for history view.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub id: String,
    pub title: String,
    pub project_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub runs: Vec<RunSummary>,
    pub total_runs: u32,
    /// Chat messages for this session (from messages.jsonl).
    pub messages: Vec<ChatMessage>,
}

/// Run detail for run view.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunDetail {
    pub summary: RunSummary,
    pub events: Vec<RunEvent>,
}

/// Storage statistics.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStats {
    pub total_sessions: usize,
    pub total_runs: usize,
    pub total_events_bytes: u64,
}

/// Role in a chat message.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    User,
    Assistant,
}

/// A single chat message stored in messages.jsonl at the session level.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

/// Entry for a single CLI session within a pipeline run.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSessionEntry {
    /// Hive-api provider session reference for resume.
    pub session_ref: String,
    pub backend: AgentBackend,
    pub model: String,
    pub stages_used: Vec<PipelineStage>,
    pub created_at: String,
    pub last_used_at: String,
}

/// File-level structure for cli_sessions.json per run.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSessionsFile {
    pub version: u32,
    /// Maps session pair name (e.g. "plan_review", "code_fix") to session entry.
    pub sessions: std::collections::HashMap<String, CliSessionEntry>,
}

impl Default for CliSessionsFile {
    fn default() -> Self {
        Self {
            version: 1,
            sessions: std::collections::HashMap::new(),
        }
    }
}
