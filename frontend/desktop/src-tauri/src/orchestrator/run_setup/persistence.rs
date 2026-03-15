//! Persistence helpers for run setup and teardown.

use tauri::AppHandle;

use crate::agents::AgentInput;
use crate::models::{RunEvent, RunStatus, *};
use crate::storage::{self, runs, sessions};

use crate::orchestrator::prompts;
use crate::orchestrator::stages::execute_run_level_agent_stage;

/// Builds context for executive summary generation.
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
        app,
        run_id,
        summary_iteration,
        PipelineStage::ExecutiveSummary,
        executive_summary_agent,
        &summary_input,
        settings,
        Some(session_id),
    )
    .await;

    // Emit executive summary artefact so the frontend can display it immediately.
    if summary_result.status == StageStatus::Completed {
        crate::orchestrator::helpers::emit_artifact(
            app,
            run_id,
            "executive_summary",
            &summary_result.output,
            summary_iteration,
        );
    }

    // Update run summary with executive summary
    if let Ok(mut summary) = runs::read_summary(run_id) {
        if summary_result.status == StageStatus::Completed {
            summary.executive_summary = Some(summary_result.output.clone());
        } else {
            summary.executive_summary = Some(format!(
                "Failed to generate summary: {}",
                summary_result.error.unwrap_or_default()
            ));
        }
        let _ = runs::update_summary(run_id, &summary);
    }
}

/// Persists the final run status to storage.
pub fn persist_final_run(run: &PipelineRun, session_id: &str) {
    // Update run summary with final state
    if let Ok(mut summary) = runs::read_summary(&run.id) {
        summary.status = run.status.clone().into();
        summary.final_verdict = run.final_verdict.clone();
        summary.current_stage = None;
        summary.current_iteration = Some(run.current_iteration);
        summary.total_iterations = run.iterations.len() as u32;
        summary.completed_at = run.completed_at.clone();
        summary.error = run.error.clone();

        // Compute files changed
        match runs::compute_files_changed(&run.id) {
            Ok(files) => summary.files_changed = files,
            Err(e) => eprintln!("Warning: Failed to compute files changed: {e}"),
        }

        if let Err(e) = runs::update_summary(&run.id, &summary) {
            eprintln!("Warning: Failed to update run summary: {e}");
        }
    }

    // Append run_end event
    let status = match run.status {
        PipelineStatus::Completed => RunStatus::Completed,
        PipelineStatus::Failed => RunStatus::Failed,
        PipelineStatus::Cancelled => RunStatus::Cancelled,
        _ => RunStatus::Completed,
    };

    let verdict = run.final_verdict.clone();
    let error = run.error.clone();

    let seq = match runs::next_sequence(&run.id) {
        Ok(s) => s,
        Err(_) => 1,
    };

    let run_end = RunEvent::RunEnd {
        v: 1,
        seq,
        ts: storage::now_rfc3339(),
        status,
        verdict,
        error,
        recovered_at: None,
    };

    if let Err(e) = runs::append_event(&run.id, run_end) {
        eprintln!("Warning: Failed to append run_end event: {e}");
    }

    // Update session metadata
    let status_str = match run.status {
        PipelineStatus::Completed => "completed",
        PipelineStatus::Failed => "failed",
        PipelineStatus::Cancelled => "cancelled",
        _ => "unknown",
    };

    let verdict_str = run.final_verdict.as_ref().map(|v| match v {
        JudgeVerdict::Complete => "COMPLETE",
        JudgeVerdict::NotComplete => "NOT COMPLETE",
    });

    if let Err(e) = sessions::touch_session(
        session_id,
        Some(&run.prompt),
        Some(status_str),
        verdict_str,
    ) {
        eprintln!("Warning: Failed to update session: {e}");
    }
}
