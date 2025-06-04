//! Document query operations

pub mod fetch;
pub mod search;

// Re-export all public functions for convenient access
pub use fetch::dash_sdk_document_fetch;
pub use search::{dash_sdk_document_search, DashSDKDocumentSearchParams};
