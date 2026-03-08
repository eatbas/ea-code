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
pub mod iteration_review;
pub mod parsing;
pub mod pipeline;
pub mod prompts;
pub mod run_setup;
pub mod stages;

pub use pipeline::run_pipeline;
