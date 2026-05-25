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
    verify_aggregate_count_proof, verify_carrier_aggregate_count_proof,
    verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, DocumentCount,
};
pub use proof::document_split_count::DocumentSplitCounts;
// Re-export `SplitCountEntry` from rs-drive at the proof-verifier
// crate root so SDK consumers don't have to depend on rs-drive
// directly just to name the entry type returned by
// `verify_distinct_count_proof` and `DocumentSplitCounts::from_verified`.
pub use drive::query::SplitCountEntry;
/// Verified average result types. Average-side analog of `DocumentSum`
/// / `DocumentSplitSums`; carry the `(count, sum)` pair the verifier
/// recovers from grovedb PR 670's `AggregateCountAndSumOnRange`
/// primitive. Client computes `avg = sum / count`.
pub use proof::document_average::{
    verify_aggregate_count_and_sum_proof, verify_carrier_aggregate_count_and_sum_proof,
    verify_distinct_count_and_sum_proof, verify_point_lookup_count_and_sum_proof,
    verify_primary_key_count_sum_tree_proof, DocumentAverage,
};
pub use proof::document_split_average::{DocumentSplitAverages, SplitAverageEntry};
/// Verified sum result types. Sum-side analogs of `DocumentCount` /
/// `DocumentSplitCounts`; see their respective module docs for the
/// grovedb PR 670 dependency status.
pub use proof::document_split_sum::{DocumentSplitSums, SplitSumEntry};
pub use proof::document_sum::{
    verify_aggregate_sum_proof, verify_carrier_aggregate_sum_proof, verify_distinct_sum_proof,
    verify_point_lookup_sum_proof, verify_primary_key_sum_tree_proof, DocumentSum,
};
// Re-export the rs-drive `SumEntry` + `AverageEntry` at the
// proof-verifier crate root, paralleling `SplitCountEntry` above —
// the per-shape verifier helpers above all return these types.
pub use drive::query::drive_document_average_query::AverageEntry;
pub use drive::query::SumEntry;
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
