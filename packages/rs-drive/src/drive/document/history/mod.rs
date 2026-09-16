//! Composite-key history queries and proof results.

mod error;
mod filter;
mod page;
mod proof;
mod query;
#[cfg(feature = "server")]
mod read;
mod verify;

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;

use error::corrupt;
pub(crate) use error::invalid;
pub use filter::DocumentHistoryFilter;
pub use page::{
    DocumentHistoryEntry, DocumentHistoryLifecycle, DocumentHistoryState, DocumentHistoryV1,
};
pub use proof::DocumentHistoryProofV1;
pub use query::DocumentHistoryQueryV1;
