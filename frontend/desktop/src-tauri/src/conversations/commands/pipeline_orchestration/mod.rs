//! Shared helpers for pipeline lifecycle management.
//!
//! Extracts the common patterns from start_pipeline, resume_pipeline,
//! and send_plan_edit_feedback to eliminate triplication.

mod coding_phase;
mod lifecycle;
mod setup;

pub(super) use coding_phase::{run_coding_phase, run_merge_chain, run_review_merge_chain};
pub(super) use lifecycle::{
    begin_pipeline_task, determine_final_status, emit_final_status, ensure_merge_stage_record,
    ensure_stage_record, pipeline_cleanup, re_emit_completed_stages,
};
pub(super) use setup::{load_pipeline_config, prepare_pipeline, prepare_pipeline_with_config};
