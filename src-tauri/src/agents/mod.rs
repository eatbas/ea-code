pub mod base;
pub mod claude;
pub mod codex;
pub mod gemini;

pub use base::{AgentInput, AgentOutput};
pub use claude::run_claude;
pub use codex::run_codex;
pub use gemini::run_gemini;
