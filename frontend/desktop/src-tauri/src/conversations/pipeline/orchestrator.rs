use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::Deserialize;
use tauri::AppHandle;

use crate::models::PipelineAgent;

use super::prompts::build_orchestrator_prompt;
use super::stage_runner::{run_stage, StageConfig};
use crate::conversations::pipeline_debug::emit_pipeline_debug;

/// Result from the orchestrator stage containing the enhanced prompt and summary.
pub struct OrchestratorResult {
    pub enhanced_prompt: String,
    pub summary: String,
}

/// JSON output structure expected from the orchestrator agent.
#[derive(Deserialize)]
struct OrchestratorOutput {
    enhanced_prompt: String,
    summary: String,
}

/// Run the orchestrator stage to enhance the user prompt and generate a summary title.
pub async fn run_orchestrator(
    app: AppHandle,
    conversation_id: String,
    workspace_path: String,
    user_prompt: String,
    orchestrator_agent: PipelineAgent,
    stage_index: usize,
    abort: Arc<AtomicBool>,
    score_id_slot: Arc<std::sync::Mutex<Option<String>>>,
    stage_buffer: Arc<std::sync::Mutex<String>>,
) -> Result<OrchestratorResult, String> {
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");

    // Create the orchestrator artefact directory.
    let prompt_enhanced_dir = format!("{conv_dir}/prompt_enhanced");
    std::fs::create_dir_all(&prompt_enhanced_dir)
        .map_err(|e| format!("Failed to create prompt_enhanced directory: {e}"))?;

    let output_path = format!("{prompt_enhanced_dir}/prompt_enhanced_output.json");
    let agent_label = format!(
        "{} / {}",
        orchestrator_agent.provider, orchestrator_agent.model
    );

    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "Orchestrator: starting with {} in mode=new, file_required=true",
            agent_label
        ),
    );

    // Run the orchestrator stage.
    let stage_result = run_stage(
        app.clone(),
        conversation_id.clone(),
        workspace_path.clone(),
        StageConfig {
            stage_index,
            stage_name: "Prompt Enhancer".to_string(),
            provider: orchestrator_agent.provider,
            model: orchestrator_agent.model,
            prompt: build_orchestrator_prompt(&user_prompt, &output_path),
            file_to_watch: output_path.clone(),
            mode: "new",
            provider_session_ref: None,
            failure_message: "Orchestrator did not produce output".to_string(),
            agent_label: agent_label.clone(),
            file_required: true,
        },
        abort,
        score_id_slot,
        stage_buffer,
    )
    .await;

    match stage_result {
        Ok(_record) => {
            // Stage completed successfully, now parse the output file.
            parse_orchestrator_output(
                &output_path,
                &user_prompt,
                &app,
                &workspace_path,
                &conversation_id,
            )
        }
        Err((_record, error)) => {
            emit_pipeline_debug(
                &app,
                &workspace_path,
                &conversation_id,
                format!("Orchestrator stage failed: {error}"),
            );
            // Return a fallback result using the original prompt.
            Ok(OrchestratorResult {
                enhanced_prompt: user_prompt.clone(),
                summary: four_word_fallback(&user_prompt),
            })
        }
    }
}

/// Parse the orchestrator output JSON file and validate the result.
fn parse_orchestrator_output(
    output_path: &str,
    user_prompt: &str,
    app: &AppHandle,
    workspace_path: &str,
    conversation_id: &str,
) -> Result<OrchestratorResult, String> {
    let output_content = match std::fs::read_to_string(output_path) {
        Ok(content) => content,
        Err(e) => {
            emit_pipeline_debug(
                app,
                workspace_path,
                conversation_id,
                format!("Orchestrator output file not found: {e}"),
            );
            return Ok(OrchestratorResult {
                enhanced_prompt: user_prompt.to_string(),
                summary: four_word_fallback(user_prompt),
            });
        }
    };

    let parsed: OrchestratorOutput = match serde_json::from_str(&output_content) {
        Ok(output) => output,
        Err(e) => {
            emit_pipeline_debug(
                app,
                workspace_path,
                conversation_id,
                format!("Failed to parse orchestrator output JSON: {e}"),
            );
            return Ok(OrchestratorResult {
                enhanced_prompt: user_prompt.to_string(),
                summary: four_word_fallback(user_prompt),
            });
        }
    };

    // Validate and sanitise the enhanced prompt.
    let enhanced_prompt = if parsed.enhanced_prompt.trim().is_empty() {
        emit_pipeline_debug(
            app,
            workspace_path,
            conversation_id,
            "Orchestrator returned empty enhanced_prompt, using original".to_string(),
        );
        user_prompt.to_string()
    } else {
        parsed.enhanced_prompt
    };

    // Validate and sanitise the summary (must be 4 words or fewer).
    let summary = if parsed.summary.trim().is_empty() {
        emit_pipeline_debug(
            app,
            workspace_path,
            conversation_id,
            "Orchestrator returned empty summary, using fallback".to_string(),
        );
        four_word_fallback(user_prompt)
    } else {
        let words: Vec<&str> = parsed.summary.split_whitespace().collect();
        if words.len() > 4 {
            // Take only the first 4 words.
            words.into_iter().take(4).collect::<Vec<_>>().join(" ")
        } else {
            parsed.summary
        }
    };

    emit_pipeline_debug(
        app,
        workspace_path,
        conversation_id,
        format!(
            "Orchestrator completed successfully with summary: {}",
            summary
        ),
    );

    Ok(OrchestratorResult {
        enhanced_prompt,
        summary,
    })
}

/// Generate a 4-word fallback title from the user prompt.
fn four_word_fallback(prompt: &str) -> String {
    prompt
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}
