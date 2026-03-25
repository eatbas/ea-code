//! Run-level agent stage execution (not tied to an iteration row).

use std::time::Instant;

use tauri::AppHandle;
use tokio::time::Duration;

use crate::agents::AgentInput;
use crate::models::*;

use super::validation::validate_text_stage_output;
pub use crate::orchestrator::helpers::{
    dispatch_agent, emit_stage, emit_stage_with_duration, resolve_stage_model,
};

/// Runs an agent stage that is not tied to an iteration row.
pub async fn execute_run_level_agent_stage(
    app: &AppHandle,
    run_id: &str,
    iteration_num: u32,
    stage: PipelineStage,
    backend: &AgentBackend,
    input: &AgentInput,
    settings: &AppSettings,
    session_id: Option<&str>,
    output_file: Option<&str>,
    cli_session_ref: Option<&str>,
) -> StageResult {
    let start = Instant::now();
    emit_stage(app, run_id, &stage, &StageStatus::Running, iteration_num);
    let model = resolve_stage_model(&stage, settings);

    let dispatch_result = if settings.agent_timeout_ms == 0 {
        dispatch_agent(
            backend,
            &model,
            input,
            settings,
            session_id,
            app,
            run_id,
            stage.clone(),
            output_file,
            cli_session_ref,
            None,
        )
        .await
    } else {
        match tokio::time::timeout(
            Duration::from_millis(settings.agent_timeout_ms),
            dispatch_agent(
                backend,
                &model,
                input,
                settings,
                session_id,
                app,
                run_id,
                stage.clone(),
                output_file,
                cli_session_ref,
                None,
            ),
        )
        .await
        {
            Ok(inner) => inner,
            Err(_) => Err(format!(
                "{stage:?} stage timed out after {} ms",
                settings.agent_timeout_ms
            )),
        }
    };

    match dispatch_result {
        Ok(dr) => {
            if matches!(stage.execution_intent(), StageExecutionIntent::Text) {
                if let Err(validation_error) =
                    validate_text_stage_output(&stage, &dr.output.raw_text)
                {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    emit_stage_with_duration(
                        app,
                        run_id,
                        &stage,
                        &StageStatus::Failed,
                        iteration_num,
                        Some(duration_ms),
                    );
                    return StageResult {
                        stage,
                        status: StageStatus::Failed,
                        output: String::new(),
                        duration_ms,
                        error: Some(validation_error),
                        backend: Some(backend.clone()),
                        provider_session_ref: dr.provider_session_ref,
                        session_pair: None,
                        resumed: None,
                    };
                }
            }
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage_with_duration(
                app,
                run_id,
                &stage,
                &StageStatus::Completed,
                iteration_num,
                Some(duration_ms),
            );
            StageResult {
                stage,
                status: StageStatus::Completed,
                output: dr.output.raw_text,
                duration_ms,
                error: None,
                backend: Some(backend.clone()),
                provider_session_ref: dr.provider_session_ref,
                session_pair: None,
                resumed: Some(cli_session_ref.is_some()),
            }
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            emit_stage_with_duration(
                app,
                run_id,
                &stage,
                &StageStatus::Failed,
                iteration_num,
                Some(duration_ms),
            );
            StageResult {
                stage,
                status: StageStatus::Failed,
                output: String::new(),
                duration_ms,
                error: Some(e),
                backend: Some(backend.clone()),
                provider_session_ref: None,
                session_pair: None,
                resumed: None,
            }
        }
    }
}
