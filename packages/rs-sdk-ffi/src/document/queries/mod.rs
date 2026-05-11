//! Document query operations

pub mod count;
pub mod fetch;
pub mod info;
pub mod search;

// Re-export all public functions for convenient access. Unified
// count entry (one function handles total/per-`In`/per-distinct-
// range modes); the prior `dash_sdk_document_split_count` was
// subsumed by exposing `return_distinct_counts_in_range` /
// `order_by_json` / `limit` on `dash_sdk_document_count`.
#[allow(unused_imports)]
pub use count::dash_sdk_document_count;
pub use fetch::dash_sdk_document_fetch;
pub use search::{dash_sdk_document_search, DashSDKDocumentSearchParams};
