//! Pipeline setup, teardown, and supporting types for the main run loop.

use tauri::{AppHandle, Emitter};

use crate::events::*;
use crate::models::*;

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
    pub review_findings: Option<ReviewFindings>,
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
            review_findings: None,
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

/// Normalises enhanced prompt output, falling back to original if needed.
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

/// Emits the final pipeline completion/error/cancellation event.
pub fn emit_final_status(app: &AppHandle, run: &PipelineRun, total_duration_ms: u64) {
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

mod persistence;
pub use persistence::{persist_final_run, run_executive_summary};
