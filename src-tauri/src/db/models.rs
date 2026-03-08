use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::*;

// ── Settings ────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = settings)]
#[serde(rename_all = "camelCase")]
pub struct SettingsRow {
    pub id: i32,
    pub claude_path: String,
    pub codex_path: String,
    pub gemini_path: String,
    pub generator_agent: String,
    pub reviewer_agent: String,
    pub fixer_agent: String,
    pub final_judge_agent: String,
    pub max_iterations: i32,
    pub require_git: bool,
    pub updated_at: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = settings)]
pub struct SettingsChangeset {
    pub claude_path: String,
    pub codex_path: String,
    pub gemini_path: String,
    pub generator_agent: String,
    pub reviewer_agent: String,
    pub fixer_agent: String,
    pub final_judge_agent: String,
    pub max_iterations: i32,
    pub require_git: bool,
    pub updated_at: String,
}

// ── Projects ────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = projects)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRow {
    pub id: i32,
    pub path: String,
    pub name: String,
    pub is_git_repo: bool,
    pub branch: Option<String>,
    pub last_opened: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject<'a> {
    pub path: &'a str,
    pub name: &'a str,
    pub is_git_repo: bool,
    pub branch: Option<&'a str>,
}

// ── Sessions ────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = sessions)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub project_id: i32,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = sessions)]
pub struct NewSession<'a> {
    pub id: &'a str,
    pub project_id: i32,
    pub title: &'a str,
}

/// Lightweight session summary for the sidebar.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub project_id: i32,
    pub run_count: i64,
    pub last_prompt: Option<String>,
    pub last_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Runs ────────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = runs)]
#[serde(rename_all = "camelCase")]
pub struct RunRow {
    pub id: String,
    pub session_id: String,
    pub prompt: String,
    pub status: String,
    pub max_iterations: i32,
    pub final_verdict: Option<String>,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = runs)]
pub struct NewRun<'a> {
    pub id: &'a str,
    pub session_id: &'a str,
    pub prompt: &'a str,
    pub max_iterations: i32,
}

/// Lightweight run summary for history lists.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub prompt: String,
    pub status: String,
    pub final_verdict: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

// ── Iterations ──────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = iterations)]
#[serde(rename_all = "camelCase")]
pub struct IterationRow {
    pub id: i32,
    pub run_id: String,
    pub number: i32,
    pub verdict: Option<String>,
    pub judge_reasoning: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = iterations)]
pub struct NewIteration<'a> {
    pub run_id: &'a str,
    pub number: i32,
}

// ── Stages ──────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = stages)]
#[serde(rename_all = "camelCase")]
pub struct StageRow {
    pub id: i32,
    pub iteration_id: i32,
    pub stage: String,
    pub status: String,
    pub output: String,
    pub duration_ms: i32,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = stages)]
pub struct NewStage<'a> {
    pub iteration_id: i32,
    pub stage: &'a str,
    pub status: &'a str,
    pub output: &'a str,
    pub duration_ms: i32,
    pub error: Option<&'a str>,
}

// ── Logs ────────────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = logs)]
#[serde(rename_all = "camelCase")]
pub struct LogRow {
    pub id: i32,
    pub run_id: String,
    pub stage: String,
    pub line: String,
    pub stream: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = logs)]
pub struct NewLog<'a> {
    pub run_id: &'a str,
    pub stage: &'a str,
    pub line: &'a str,
    pub stream: &'a str,
}

// ── Artefacts ───────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = artifacts)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRow {
    pub id: i32,
    pub run_id: String,
    pub iteration: i32,
    pub kind: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[diesel(table_name = artifacts)]
pub struct NewArtifact<'a> {
    pub run_id: &'a str,
    pub iteration: i32,
    pub kind: &'a str,
    pub content: &'a str,
}

// ── Questions ───────────────────────────────────────────────────────────

#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = questions)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRow {
    pub id: String,
    pub run_id: String,
    pub stage: String,
    pub iteration: i32,
    pub question_text: String,
    pub agent_output: String,
    pub optional: bool,
    pub answer: Option<String>,
    pub skipped: bool,
    pub asked_at: String,
    pub answered_at: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = questions)]
pub struct NewQuestion<'a> {
    pub id: &'a str,
    pub run_id: &'a str,
    pub stage: &'a str,
    pub iteration: i32,
    pub question_text: &'a str,
    pub agent_output: &'a str,
    pub optional: bool,
}

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
    pub max_iterations: i32,
    pub started_at: String,
    pub completed_at: Option<String>,
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
    pub stages: Vec<StageRow>,
}
