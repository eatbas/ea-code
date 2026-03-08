/// Database model types, split by domain area.
///
/// All public types are re-exported here so that callers can continue to
/// use `crate::db::models::*` without change.

mod details;
mod records;
mod settings;

pub use details::*;
pub use records::*;
pub use settings::*;
