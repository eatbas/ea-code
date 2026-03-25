//! Utility functions for the orchestration pipeline:
//! agent dispatch, event emission, cancellation, and context persistence.
//!
//! Split into focused submodules for maintainability:
//! - `dispatch` — agent dispatch routing and artifact matching
//! - `emission` — IPC event emission helpers
//! - `file_refs` — file reference builders and model resolution
//! - `interrupts` — cancellation, pause, and interrupt handling
//! - `watchdog` — per-agent file watchdog for text stages
//! - `session_tracker` — CLI session reference tracker

pub mod dispatch;
pub mod emission;
pub mod events;
pub mod file_refs;
pub mod interrupts;
pub mod session_tracker;
pub mod watchdog;

// Re-export all public items so existing `use crate::orchestrator::helpers::X`
// paths continue to work without changes.

pub use dispatch::dispatch_agent;
pub use emission::{
    emit_artifact, emit_prompt_artifact, emit_stage, emit_stage_with_duration, epoch_millis,
};
pub use file_refs::{
    artifact_file_path, build_iteration_refs, descriptive_artifact_name, file_ref,
    file_ref_or_inline, resolve_stage_model,
};
pub use interrupts::{
    is_cancelled, push_cancel_iteration, wait_for_cancel, wait_for_interrupt, wait_if_paused,
    RunInterrupt,
};
pub use events::handle_stage_failure;
pub use session_tracker::CliSessionTracker;
pub use watchdog::{per_agent_file_watchdog, slot_prefix_from_output_kind};
