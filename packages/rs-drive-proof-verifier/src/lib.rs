//! Proof verification library for Dash Drive
#![warn(missing_docs)]
#![allow(clippy::result_large_err)]

/// Errors that can occur during proof verification
pub mod error;
/// Implementation of proof verification
mod proof;
pub mod types;
mod verify;
pub use error::Error;
pub use proof::document_count::{
    verify_aggregate_count_proof, verify_distinct_count_proof, DocumentCount,
};
pub use proof::document_split_count::DocumentSplitCounts;
// Re-export `SplitCountEntry` from rs-drive at the proof-verifier
// crate root so SDK consumers don't have to depend on rs-drive
// directly just to name the entry type returned by
// `verify_distinct_count_proof` and `DocumentSplitCounts::from_verified`.
pub use drive::query::SplitCountEntry;
pub use proof::{FromProof, Length};

// Re-export context provider types from dash-context-provider
#[cfg(feature = "mocks")]
pub use dash_context_provider::MockContextProvider;
pub use dash_context_provider::{ContextProvider, ContextProviderError, DataContractProvider};

/// From Request
pub mod from_request;
/// Implementation of unproved verification
pub mod unproved;

// Needed for #[derive(PlatformSerialize, PlatformDeserialize)]
#[cfg(feature = "mocks")]
use dpp::serialization;
