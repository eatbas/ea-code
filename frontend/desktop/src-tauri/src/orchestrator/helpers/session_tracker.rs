//! CLI session reference tracker for context continuity across stages.

use std::collections::HashMap;

use crate::models::*;

use super::dispatch::session_pair_for_stage;

/// Tracks CLI session references across pipeline stages within a run.
///
/// Session refs are looked up before dispatch and stored after.
/// The tracker persists to `cli_sessions.json` after each update.
///
/// Per-slot pairing: Planner[0] stores under "plan_review_0", and
/// Reviewer[0] looks up "plan_review_0" to resume the same CLI session.
/// Backend is stored alongside the ref so a Claude session is not
/// accidentally passed to a Gemini agent.
pub struct CliSessionTracker {
    workspace_path: String,
    session_id: String,
    run_id: String,
    /// pair_key -> (backend, provider_session_ref)
    sessions: HashMap<String, (AgentBackend, String)>,
}

impl CliSessionTracker {
    pub fn new(workspace_path: String, session_id: String, run_id: String) -> Self {
        Self {
            workspace_path,
            session_id,
            run_id,
            sessions: HashMap::new(),
        }
    }

    /// Gets the session ref for a stage's pair, if one exists and the backend matches.
    ///
    /// Returns `None` when the stored session was created by a different backend
    /// (e.g. a Claude session cannot be resumed by Gemini).
    pub fn get_ref_for_stage(&self, stage: &PipelineStage, backend: &AgentBackend) -> Option<&str> {
        let pair = session_pair_for_stage(stage);
        self.sessions.get(&pair).and_then(|(stored_backend, ref_str)| {
            if stored_backend == backend {
                Some(ref_str.as_str())
            } else {
                None
            }
        })
    }

    /// Stores a session ref returned from a stage and persists to disk.
    ///
    /// Uses the backend embedded in the `StageResult`. Skips storage if backend
    /// or session ref is absent (e.g. skipped stages).
    pub fn store_ref_from_result(&mut self, result: &StageResult) {
        let backend = match result.backend.as_ref() {
            Some(b) => b,
            None => return,
        };
        if let Some(ref session_ref) = result.provider_session_ref {
            let pair = session_pair_for_stage(&result.stage);
            self.sessions.insert(pair.clone(), (backend.clone(), session_ref.clone()));

            // Persist to disk (best-effort)
            let entry = CliSessionEntry {
                session_ref: session_ref.clone(),
                backend: backend.clone(),
                model: String::new(),
                stages_used: vec![result.stage.clone()],
                created_at: crate::storage::now_rfc3339(),
                last_used_at: crate::storage::now_rfc3339(),
            };
            if let Err(e) = crate::storage::runs::update_cli_session(
                &self.workspace_path,
                &self.session_id,
                &self.run_id,
                &pair,
                entry,
            ) {
                eprintln!("Warning: Failed to persist CLI session ref for {pair}: {e}");
            }
        }
    }
}
