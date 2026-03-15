use serde::{Deserialize, Serialize};

use super::pipeline::{JudgeVerdict, PipelineStage};

/// Run status for terminal events.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    Failed,
    Cancelled,
    Crashed,
}

/// Stage end status variants.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageEndStatus {
    Completed,
    Failed,
    Skipped,
}

/// Plan audit verdict variants.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PlanAuditVerdict {
    #[serde(rename = "APPROVED")]
    Approved,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "NEEDS_REVISION")]
    NeedsRevision,
}

/// Individual run event for events.jsonl.
///
/// Every event includes:
/// - `v`: schema version (u32)
/// - `seq`: monotonic sequence number (u64)
/// - `ts`: RFC 3339 timestamp
///
/// The `type` field discriminates the variant.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    /// Marks the beginning of a pipeline run.
    #[serde(rename_all = "camelCase")]
    RunStart {
        v: u32,
        seq: u64,
        ts: String,
        prompt: String,
        max_iterations: u32,
    },

    /// A stage begins execution.
    #[serde(rename_all = "camelCase")]
    StageStart {
        v: u32,
        seq: u64,
        ts: String,
        stage: PipelineStage,
        iteration: u32,
    },

    /// A stage finishes execution.
    #[serde(rename_all = "camelCase")]
    StageEnd {
        v: u32,
        seq: u64,
        ts: String,
        stage: PipelineStage,
        iteration: u32,
        status: StageEndStatus,
        /// Duration in milliseconds.
        duration_ms: u64,
        /// Plan audit verdict, if this was a plan_audit stage.
        #[serde(skip_serializing_if = "Option::is_none")]
        audit_verdict: Option<PlanAuditVerdict>,
        /// Judge verdict, if this was a judge stage.
        #[serde(skip_serializing_if = "Option::is_none")]
        verdict: Option<JudgeVerdict>,
    },

    /// An iteration loop completes with judge verdict.
    #[serde(rename_all = "camelCase")]
    IterationEnd {
        v: u32,
        seq: u64,
        ts: String,
        iteration: u32,
        verdict: JudgeVerdict,
    },

    /// User answered a question during the run.
    #[serde(rename_all = "camelCase")]
    Question {
        v: u32,
        seq: u64,
        ts: String,
        stage: PipelineStage,
        iteration: u32,
        question: String,
        answer: String,
        skipped: bool,
    },

    /// Terminal event - run completed, failed, or cancelled.
    #[serde(rename_all = "camelCase")]
    RunEnd {
        v: u32,
        seq: u64,
        ts: String,
        status: RunStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        verdict: Option<JudgeVerdict>,
        /// Error message when status is failed or crashed.
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        /// Recovery timestamp for crash recovery events.
        #[serde(skip_serializing_if = "Option::is_none")]
        recovered_at: Option<String>,
    },
}

impl RunEvent {
    /// Returns the schema version of this event.
    pub fn schema_version(&self) -> u32 {
        match self {
            RunEvent::RunStart { v, .. } => *v,
            RunEvent::StageStart { v, .. } => *v,
            RunEvent::StageEnd { v, .. } => *v,
            RunEvent::IterationEnd { v, .. } => *v,
            RunEvent::Question { v, .. } => *v,
            RunEvent::RunEnd { v, .. } => *v,
        }
    }

    /// Returns the sequence number of this event.
    pub fn sequence(&self) -> u64 {
        match self {
            RunEvent::RunStart { seq, .. } => *seq,
            RunEvent::StageStart { seq, .. } => *seq,
            RunEvent::StageEnd { seq, .. } => *seq,
            RunEvent::IterationEnd { seq, .. } => *seq,
            RunEvent::Question { seq, .. } => *seq,
            RunEvent::RunEnd { seq, .. } => *seq,
        }
    }

    /// Returns the timestamp of this event.
    pub fn timestamp(&self) -> &str {
        match self {
            RunEvent::RunStart { ts, .. } => ts,
            RunEvent::StageStart { ts, .. } => ts,
            RunEvent::StageEnd { ts, .. } => ts,
            RunEvent::IterationEnd { ts, .. } => ts,
            RunEvent::Question { ts, .. } => ts,
            RunEvent::RunEnd { ts, .. } => ts,
        }
    }

    /// Returns true if this is a terminal event (run_end).
    pub fn is_terminal(&self) -> bool {
        matches!(self, RunEvent::RunEnd { .. })
    }
}
