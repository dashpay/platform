//! Document query operations

pub mod count;
pub mod fetch;
pub mod info;
pub mod search;
/// Sum-side FFI entry. Skeleton — lights up alongside grovedb PR 670.
pub mod sum;

pub use count::dash_sdk_document_count;
pub use fetch::dash_sdk_document_fetch;
pub use search::{dash_sdk_document_search, DashSDKDocumentSearchParams};
// `sum::dash_sdk_document_sum` is exported via its `#[no_mangle] extern "C"`
// declaration; no re-export needed (and clippy flags the re-export as unused
// because nothing inside the crate calls it by path).
