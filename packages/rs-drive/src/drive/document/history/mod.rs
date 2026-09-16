//! Composite-key history queries and proof results.

mod error;
mod filter;
mod page;
mod proof;
mod query;

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;

pub(crate) use error::{corrupt, invalid};
pub use filter::DocumentHistoryFilter;
pub use page::{
    DocumentHistoryEntry, DocumentHistoryLifecycle, DocumentHistoryState, DocumentHistoryV1,
};
pub use proof::{DocumentHistoryProof, DocumentHistoryProofV1};
pub use query::DocumentHistoryQueryV1;
