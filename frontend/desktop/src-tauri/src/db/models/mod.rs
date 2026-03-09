/// Database model types, split by domain area.
///
/// All public types are re-exported here so that callers can continue to
/// use `crate::db::models::*` without change.

mod details;
mod mcp;
mod records;
mod settings;
mod skills;

pub use details::*;
pub use mcp::*;
pub use records::*;
pub use settings::*;
pub use skills::*;
