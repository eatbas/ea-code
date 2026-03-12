//! Pipeline setup, teardown, and supporting types for the main run loop.

use tauri::{AppHandle, Emitter};

use crate::agents::AgentInput;
use crate::db::{self, DbPool};
use crate::events::*;
use crate::models::*;

use super::helpers::*;
use super::prompts;
use super::stages::*;

/// Tracks accumulated context within a single iteration.
#[derive(Clone, Debug)]
pub struct IterationContext {
    pub original_prompt: String,
    pub enhanced_prompt: String,
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
}

impl IterationContext {
    pub fn new(original_prompt: String) -> Self {
        Self {
            original_prompt: original_prompt.clone(),
            enhanced_prompt: original_prompt,
            planner_plan: None,
            audit_verdict: None,
            audit_reasoning: None,
            audited_plan: None,
            review_output: None,
            review_user_guidance: None,
            fix_output: None,
            judge_output: None,
            generate_question: None,
            generate_answer: None,
            fix_question: None,
            fix_answer: None,
        }
    }

    pub fn selected_plan(&self) -> Option<&str> {
        self.audited_plan
            .as_deref()
            .or(self.planner_plan.as_deref())
    }
}

/// Known CLI stderr noise patterns that should be stripped from agent output.
const STDERR_NOISE_PATTERNS: &[&str] = &[
    "YOLO mode",
    "Approval mode \"plan\" is only available when experimental.plan is enabled",
    "Loaded cached credentials",
    "[ERROR] [IDEClient]",
    "Failed to connect to IDE companion extension",
    "/ide install",
    "supports tool updates",
    "Listening for changes",
    "Server 'context7'",
    "[stderr]",
];

fn looks_like_enhancer_execution_log(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    lower.starts_with("function.")
        || lower.starts_with("i will now ")
        || lower.starts_with("i'll now ")
        || lower.starts_with("i have implemented")
        || lower.starts_with("i've implemented")
        || lower.contains("\ni will now ")
        || lower.contains("\ni'll now ")
        || lower.contains("\ni have implemented")
        || lower.contains("\ni've implemented")
}

/// Removes known CLI informational/noise lines from agent output.
fn strip_cli_noise(output: &str) -> String {
    output
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return true;
            }
            !STDERR_NOISE_PATTERNS
                .iter()
                .any(|pattern| trimmed.contains(pattern))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn normalise_enhanced_prompt(enhanced_output: &str, fallback_prompt: &str) -> String {
    let cleaned = strip_cli_noise(enhanced_output);
    let candidate = cleaned.trim();
    if candidate.is_empty() {
        fallback_prompt.to_string()
    } else if looks_like_enhancer_execution_log(candidate) {
        fallback_prompt.to_string()
    } else {
        candidate.to_string()
    }
}

/// Appends shared run context to a stage system prompt.
pub fn compose_agent_context(system_prompt: String, shared_context: &str) -> String {
    let trimmed = shared_context.trim();
    if trimmed.is_empty() {
        return system_prompt;
    }
    format!("{system_prompt}\n\n--- Context ---\n{trimmed}")
}

#[allow(dead_code)]
pub fn build_executive_summary_context(run: &PipelineRun) -> String {
    let mut lines = vec![
        format!("Run ID: {}", run.id),
        format!("Prompt: {}", run.prompt),
        format!("Status: {:?}", run.status),
        format!("Max Iterations: {}", run.max_iterations),
        format!("Completed Iterations: {}", run.current_iteration),
    ];
    if let Some(verdict) = run.final_verdict.as_ref() {
        lines.push(format!("Final Verdict: {verdict:?}"));
    }
    if let Some(error) = run.error.as_ref() {
        lines.push(format!("Error: {error}"));
    }
    for iteration in &run.iterations {
        lines.push(format!("Iteration {}:", iteration.number));
        for stage in &iteration.stages {
            lines.push(format!(
                "- {:?}: {:?} ({}ms)",
                stage.stage, stage.status, stage.duration_ms
            ));
        }
        if let Some(reasoning) = iteration.judge_reasoning.as_ref() {
            lines.push(format!("Judge Reasoning: {reasoning}"));
        }
    }
    lines.join("\n")
}

/// Runs the executive summary stage and persists results.
#[allow(dead_code)]
pub async fn run_executive_summary(
    app: &AppHandle,
    run_id: &str,
    run: &PipelineRun,
    settings: &AppSettings,
    session_id: &str,
    db: &DbPool,
) {
    let Some(executive_summary_agent) = settings.executive_summary_agent.as_ref() else {
        return;
    };

    let summary_iteration = run.current_iteration;
    let summary_input = AgentInput {
        prompt: prompts::build_executive_summary_system(),
        context: Some(build_executive_summary_context(run)),
        workspace_path: run.workspace_path.clone(),
    };
    let summary_result = execute_run_level_agent_stage(
        app, run_id, summary_iteration, PipelineStage::ExecutiveSummary,
        executive_summary_agent, &summary_input, settings,
        Some(session_id), db,
    )
    .await;
    let summary_model = resolve_stage_model(&PipelineStage::ExecutiveSummary, settings);
    let summary_generated_at = crate::db::now_rfc3339();
    let summary_status = if summary_result.status == StageStatus::Completed {
        "completed"
    } else {
        "failed"
    };
    let patch = db::runs::RunExecutiveSummaryPatch {
        executive_summary: if summary_result.status == StageStatus::Completed {
            Some(summary_result.output.as_str())
        } else {
            None
        },
        executive_summary_status: Some(summary_status),
        executive_summary_error: summary_result.error.as_deref(),
        executive_summary_agent: Some(backend_to_db_str(executive_summary_agent)),
        executive_summary_model: Some(summary_model.as_str()),
        executive_summary_generated_at: Some(summary_generated_at.as_str()),
    };
    let _ = db::runs::update_executive_summary(db, run_id, &patch);
    if summary_result.status == StageStatus::Completed {
        emit_artifact(
            app, run_id, "executive_summary", &summary_result.output,
            summary_iteration, db,
        );
    }
}

/// Emits the final pipeline completion/error/cancellation event.
pub fn emit_final_status(
    app: &AppHandle,
    run: &PipelineRun,
    total_duration_ms: u64,
) {
    match &run.status {
        PipelineStatus::Completed => {
            let _ = app.emit(
                EVENT_PIPELINE_COMPLETED,
                PipelineCompletedPayload {
                    run_id: run.id.clone(),
                    verdict: run.final_verdict.clone().unwrap_or(JudgeVerdict::NotComplete),
                    total_iterations: run.current_iteration,
                    duration_ms: total_duration_ms,
                },
            );
        }
        PipelineStatus::Failed => {
            let _ = app.emit(
                EVENT_PIPELINE_ERROR,
                PipelineErrorPayload {
                    run_id: run.id.clone(),
                    stage: None,
                    message: run.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                },
            );
        }
        PipelineStatus::Cancelled => {
            let _ = app.emit(
                EVENT_PIPELINE_ERROR,
                PipelineErrorPayload {
                    run_id: run.id.clone(),
                    stage: None,
                    message: "Pipeline cancelled by user".to_string(),
                },
            );
        }
        _ => {}
    }
}

/// Persists the final run status to the database.
pub fn persist_final_run(
    db: &DbPool,
    run: &PipelineRun,
    session_id: &str,
) {
    let status_str = match &run.status {
        PipelineStatus::Completed => "completed",
        PipelineStatus::Failed => "failed",
        PipelineStatus::Cancelled => "cancelled",
        PipelineStatus::Paused => "paused",
        _ => "completed",
    };
    let verdict_str = run.final_verdict.as_ref().map(|v| match v {
        JudgeVerdict::Complete => "COMPLETE",
        JudgeVerdict::NotComplete => "NOT COMPLETE",
    });
    let _ = db::runs::complete(db, &run.id, status_str, verdict_str, run.error.as_deref());
    let _ = db::sessions::touch(db, session_id);
}
