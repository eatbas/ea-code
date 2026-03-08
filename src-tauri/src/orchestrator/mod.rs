//! Orchestration pipeline module.
//!
//! Split into sub-modules for maintainability:
//! - `prompts` — v2.5.0 system prompts for each pipeline stage
//! - `helpers` — utility functions (dispatch, events, cancellation)
//! - `stages` — stage execution functions
//! - `parsing` — verdict and plan audit output parsing
//! - `run_setup` — pipeline setup, teardown, and supporting types
//! - `pipeline` — main `run_pipeline` loop

pub mod helpers;
pub mod iteration;
pub mod iteration_planning;
pub mod iteration_review;
pub mod context_summary;
pub mod parsing;
pub mod plan_gate;
pub mod pipeline;
pub mod prompts;
pub mod run_setup;
pub mod skill_selection;
pub mod skill_stage;
pub mod stages;
pub mod user_questions;

pub use pipeline::run_pipeline;
