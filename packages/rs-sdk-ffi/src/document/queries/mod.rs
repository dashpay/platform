//! Document query operations

pub mod count;
pub mod fetch;
pub mod info;
pub mod search;

// Re-export all public functions for convenient access
#[allow(unused_imports)]
pub use count::{dash_sdk_document_count, dash_sdk_document_split_count};
pub use fetch::dash_sdk_document_fetch;
pub use search::{dash_sdk_document_search, DashSDKDocumentSearchParams};
