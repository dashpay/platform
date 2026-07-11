//! Document query operations

/// Average-side FFI entry (`dash_sdk_document_average`). Wraps the
/// rs-sdk `DocumentSplitAverages::fetch` flow.
pub mod average;
pub mod count;
pub mod fetch;
pub mod info;
pub mod search;
/// Sum-side FFI entry (`dash_sdk_document_sum`). Wraps the rs-sdk
/// `DocumentSplitSums::fetch` flow.
pub mod sum;

pub use count::dash_sdk_document_count;
pub use fetch::dash_sdk_document_fetch;
pub use search::{dash_sdk_document_search, DashSDKDocumentSearchParams};
// `sum::dash_sdk_document_sum` and `average::dash_sdk_document_average`
// are exported via their `#[no_mangle] extern "C"` declarations; no
// re-export needed (and clippy flags re-exports as unused because nothing
// inside the crate calls them by path).
