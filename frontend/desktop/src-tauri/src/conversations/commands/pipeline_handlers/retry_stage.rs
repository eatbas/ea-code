//! Manually re-run a single failed pipeline stage by resuming its
//! captured provider session with a `continue` turn.
//!
//! Unlike [`super::resume::resume_pipeline`], this handler does not
//! re-issue the original prompt. It assumes the agent already saw the
//! prompt during the failed attempt and only needs to be nudged to
//! finish — which is the right call when the failure was a transient
//! provider-side connection error rather than a semantic problem.
//!
//! The pipeline orchestration is *not* advanced after the retry: even
//! on success, the conversation stays at its current stage so the user
//! can decide whether to click `Resume Pipeline` to continue the
//! remaining stages or kick off another retry.

use tauri::AppHandle;

use crate::conversations::pipeline::stage_runner::{run_stage, StageConfig};
use crate::conversations::pipeline_debug::emit_pipeline_debug;
use crate::models::{ConversationDetail, ConversationStatus, PipelineStageRecord};

use super::super::super::persistence;
use super::super::pipeline_orchestration::{
    begin_pipeline_task, emit_final_status, pipeline_cleanup,
};

/// Re-run a single failed pipeline stage by resuming its captured
/// provider session with a `continue` prompt. Returns the conversation
/// detail in its updated `Running` state so the frontend can transition
/// back to the live pipeline view immediately.
#[tauri::command]
pub async fn retry_failed_stage(
    app: AppHandle,
    workspace_path: String,
    conversation_id: String,
    stage_index: usize,
) -> Result<ConversationDetail, String> {
    let detail = persistence::get_conversation(&workspace_path, &conversation_id)?;
    if detail.summary.status == ConversationStatus::Running {
        return Err("Pipeline is already running".to_string());
    }

    let state = persistence::load_pipeline_state(&workspace_path, &conversation_id)?
        .ok_or("No pipeline state found for this conversation")?;

    let stage = state
        .stages
        .iter()
        .find(|s| s.stage_index == stage_index)
        .cloned()
        .ok_or_else(|| format!("No stage at index {stage_index}"))?;

    if stage.status != ConversationStatus::Failed {
        return Err(format!(
            "Stage `{}` is not in failed state ({:?})",
            stage.stage_name, stage.status,
        ));
    }

    let session_ref = stage
        .provider_session_ref
        .clone()
        .ok_or_else(|| {
            format!(
                "Stage `{}` has no captured provider session — cannot send `continue`",
                stage.stage_name,
            )
        })?;

    let stage_config = build_retry_stage_config(&workspace_path, &conversation_id, &stage, session_ref.clone())?;

    // Make the runtime registries sticky for this retry so the existing
    // stage runner code paths can persist score ids and stream output
    // into the same buffers the rest of the app reads from.
    let score_slot =
        persistence::ensure_pipeline_score_slot(&workspace_path, &conversation_id, stage_index)?;
    let stage_buffer =
        persistence::ensure_pipeline_stage_buffer(&workspace_path, &conversation_id, stage_index)?;
    if let Ok(mut buf) = stage_buffer.lock() {
        buf.clear();
    }
    if let Ok(mut slot) = score_slot.lock() {
        *slot = None;
    }

    let abort = persistence::register_abort_flag(&workspace_path, &conversation_id)?;
    abort.store(false, std::sync::atomic::Ordering::Release);

    let guard = begin_pipeline_task(&app, &workspace_path, &conversation_id)
        .ok_or("Conversation is already running".to_string())?;
    let running_detail = persistence::get_conversation(&workspace_path, &conversation_id)?;

    emit_pipeline_debug(
        &app,
        &workspace_path,
        &conversation_id,
        format!(
            "Manual retry requested for stage `{}` (index {stage_index}); resuming session {session_ref}",
            stage.stage_name,
        ),
    );

    let app_handle = app.clone();
    let ws = workspace_path.clone();
    let conv_id = conversation_id.clone();
    let stage_name_for_task = stage.stage_name.clone();

    tokio::spawn(async move {
        let _guard = guard;

        let result = run_stage(
            app_handle.clone(),
            conv_id.clone(),
            ws.clone(),
            stage_config,
            abort,
            score_slot,
            stage_buffer,
        )
        .await;

        let (status, error) = match result {
            Ok(_) => {
                // The stage now has a completed marker, but the rest of
                // the pipeline beyond it has not been run — leave the
                // conversation in a not-running terminal state so the
                // frontend can offer Resume Pipeline. We deliberately
                // mirror the failure state's bucket so the existing
                // canResume path lights up.
                emit_pipeline_debug(
                    &app_handle,
                    &ws,
                    &conv_id,
                    format!(
                        "Manual retry succeeded for stage `{stage_name_for_task}`; click Resume Pipeline to continue",
                    ),
                );
                (ConversationStatus::Stopped, None)
            }
            Err((_, error)) => {
                emit_pipeline_debug(
                    &app_handle,
                    &ws,
                    &conv_id,
                    format!("Manual retry failed for stage `{stage_name_for_task}`: {error}"),
                );
                (ConversationStatus::Failed, Some(error))
            }
        };

        emit_final_status(&app_handle, &ws, &conv_id, status, error);
        pipeline_cleanup(&ws, &conv_id);
    });

    Ok(running_detail)
}

/// Build the StageConfig for a retry. Reuses the original stage's
/// agent + watched-file definition but forces `mode=resume`,
/// `prompt=continue`, and the captured `provider_session_ref` so the
/// agent picks up exactly where it stopped. The agent provider/model
/// are inferred from the stored `agent_label` (which has the
/// `provider / model` shape).
fn build_retry_stage_config(
    workspace_path: &str,
    conversation_id: &str,
    stage: &PipelineStageRecord,
    session_ref: String,
) -> Result<StageConfig, String> {
    let (provider, model) = parse_agent_label(&stage.agent_label).ok_or_else(|| {
        format!(
            "Stage `{}` has malformed agent label `{}`",
            stage.stage_name, stage.agent_label,
        )
    })?;

    let file_to_watch = derive_watched_file(workspace_path, conversation_id, &stage.stage_name)?;
    // Most pipeline stages produce an explicit summary file; the Coder
    // family is the only one we treat as "marker optional" because
    // codebase mutations are themselves the artefact.
    let file_required = !is_code_stage(&stage.stage_name);

    Ok(StageConfig {
        stage_index: stage.stage_index,
        stage_name: stage.stage_name.clone(),
        provider,
        model,
        prompt: "continue".to_string(),
        file_to_watch,
        mode: "resume",
        provider_session_ref: Some(session_ref),
        failure_message: format!("{} retry did not produce a completion summary", stage.stage_name),
        agent_label: stage.agent_label.clone(),
        file_required,
    })
}

fn parse_agent_label(label: &str) -> Option<(String, String)> {
    let mut parts = label.splitn(2, '/');
    let provider = parts.next()?.trim();
    let model = parts.next()?.trim();
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    Some((provider.to_string(), model.to_string()))
}

fn is_code_stage(stage_name: &str) -> bool {
    stage_name == "Coder"
        || stage_name.starts_with("Code Fixer")
}

/// Reconstruct the watched-file path that the original stage runner
/// would have used. Mirrors the per-stage layout in
/// `frontend/desktop/src-tauri/src/conversations/pipeline/*.rs`.
fn derive_watched_file(
    workspace_path: &str,
    conversation_id: &str,
    stage_name: &str,
) -> Result<String, String> {
    let conv_dir = format!("{workspace_path}/.maestro/conversations/{conversation_id}");

    if stage_name == "Prompt Enhancer" {
        return Ok(format!("{conv_dir}/prompt_enhanced/prompt_enhanced_output.json"));
    }
    if stage_name == "Plan Merge" {
        return Ok(format!("{conv_dir}/plan_merged/plan_merged.md"));
    }
    if stage_name == "Coder" {
        return Ok(format!("{conv_dir}/coder/coder_done.md"));
    }
    if stage_name == "Review Merge" {
        return Ok(format!("{conv_dir}/review_merged/review_merged.md"));
    }
    if stage_name == "Code Fixer" {
        return Ok(format!("{conv_dir}/code_fixer/code_fixer_done.md"));
    }
    if let Some(rest) = stage_name.strip_prefix("Planner ") {
        let n: usize = rest.trim().parse().map_err(|_| {
            format!("Could not parse planner index from stage name `{stage_name}`")
        })?;
        return Ok(format!("{conv_dir}/plan/Plan-{n}.md"));
    }
    if let Some(rest) = stage_name.strip_prefix("Reviewer ") {
        // Reviewer 1, Reviewer 2, ... or with trailing `(Cycle N)` suffix.
        let (number_part, cycle_suffix) = match rest.find(" (Cycle ") {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        let n: usize = number_part.trim().parse().map_err(|_| {
            format!("Could not parse reviewer index from stage name `{stage_name}`")
        })?;
        let review_dir = if cycle_suffix.is_empty() {
            format!("{conv_dir}/review")
        } else {
            // " (Cycle 2)" -> "review_2"
            let cycle_num: String = cycle_suffix
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            format!("{conv_dir}/review_{cycle_num}")
        };
        return Ok(format!("{review_dir}/Review-{n}.md"));
    }
    if let Some(rest) = stage_name.strip_prefix("Review Merge") {
        // Cycle suffix only.
        let cycle_num: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
        if cycle_num.is_empty() {
            return Ok(format!("{conv_dir}/review_merged/review_merged.md"));
        }
        return Ok(format!("{conv_dir}/review_merged_{cycle_num}/review_merged.md"));
    }
    if let Some(rest) = stage_name.strip_prefix("Code Fixer") {
        let cycle_num: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
        if cycle_num.is_empty() {
            return Ok(format!("{conv_dir}/code_fixer/code_fixer_done.md"));
        }
        return Ok(format!("{conv_dir}/code_fixer_{cycle_num}/code_fixer_done.md"));
    }

    Err(format!(
        "Don't know how to derive watched-file path for stage `{stage_name}`",
    ))
}

#[cfg(test)]
mod tests {
    use super::{derive_watched_file, is_code_stage, parse_agent_label};

    #[test]
    fn parses_agent_label() {
        assert_eq!(
            parse_agent_label("kimi / kimi-code/kimi-for-coding"),
            Some(("kimi".to_string(), "kimi-code/kimi-for-coding".to_string())),
        );
    }

    #[test]
    fn rejects_malformed_agent_label() {
        assert!(parse_agent_label("kimi-only").is_none());
        assert!(parse_agent_label(" / model").is_none());
        assert!(parse_agent_label("provider / ").is_none());
    }

    #[test]
    fn classifies_code_stages() {
        assert!(is_code_stage("Coder"));
        assert!(is_code_stage("Code Fixer"));
        assert!(is_code_stage("Code Fixer (Cycle 2)"));
        assert!(!is_code_stage("Planner 1"));
        assert!(!is_code_stage("Reviewer 1"));
    }

    #[test]
    fn derives_first_run_paths() {
        let p = derive_watched_file("/ws", "abc", "Planner 2").unwrap();
        assert_eq!(p, "/ws/.maestro/conversations/abc/plan/Plan-2.md");

        let p = derive_watched_file("/ws", "abc", "Reviewer 1").unwrap();
        assert_eq!(p, "/ws/.maestro/conversations/abc/review/Review-1.md");

        let p = derive_watched_file("/ws", "abc", "Coder").unwrap();
        assert_eq!(p, "/ws/.maestro/conversations/abc/coder/coder_done.md");
    }

    #[test]
    fn derives_redo_cycle_paths() {
        let p = derive_watched_file("/ws", "abc", "Reviewer 1 (Cycle 2)").unwrap();
        assert_eq!(p, "/ws/.maestro/conversations/abc/review_2/Review-1.md");

        let p = derive_watched_file("/ws", "abc", "Review Merge (Cycle 2)").unwrap();
        assert_eq!(
            p,
            "/ws/.maestro/conversations/abc/review_merged_2/review_merged.md",
        );

        let p = derive_watched_file("/ws", "abc", "Code Fixer (Cycle 3)").unwrap();
        assert_eq!(
            p,
            "/ws/.maestro/conversations/abc/code_fixer_3/code_fixer_done.md",
        );
    }
}
