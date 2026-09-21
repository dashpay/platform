//! The per-type tree that holds the lifecycle records of keep-history
//! documents, and the reads over it.
//!
//! The record itself is `dpp::document::lifecycle::DocumentLifecycleRecord`.
//! It lives in its own tree under the document type rather than inside the
//! document's history, so every key in the history stays in the revision
//! domain and deleted documents stay enumerable per type.

#[cfg(feature = "server")]
mod fetch;

#[cfg(feature = "server")]
mod refund_recipients;

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;

#[cfg(feature = "server")]
pub use fetch::DocumentLifecycleState;
