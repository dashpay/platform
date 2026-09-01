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
pub use proof::chained_document::{
    verify_chained_documents_proof as verify_chained_documents_tenderdash_proof, ChainedDocuments,
};
pub use proof::document_count::{
    verify_aggregate_count_proof, verify_carrier_aggregate_count_proof,
    verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, DocumentCount,
};
/// Verified having-range (`GROUP BY … HAVING <aggregate> <op> <value>
/// LIMIT n`) result types. `DocumentHavingEntries` carries one entry
/// per matching group **in axis order**;
/// [`verify_having_range_proof`] is the tenderdash-composition wrapper
/// that binds the proof's reconstructed root hash to the signed app
/// hash and returns the verified entry list — including its
/// completeness: an in-range group the node omitted fails verification.
pub use proof::document_having::{verify_having_range_proof, DocumentHavingEntries};
/// Verified ranked (`GROUP BY … ORDER BY <aggregate> LIMIT n
/// [OFFSET m]`) result types. `DocumentRankedEntries` carries one entry
/// per returned group **in ranking order**, plus the `starting_rank`
/// that pins each entry to an absolute position;
/// [`verify_ranked_top_k_proof`] is the tenderdash-composition wrapper
/// that binds the proof's reconstructed root hash to the signed app
/// hash and returns the whole verified [`drive::query::RankedPage`].
pub use proof::document_ranked::{verify_ranked_top_k_proof, DocumentRankedEntries};
pub use proof::document_split_count::DocumentSplitCounts;
// Re-export `SplitCountEntry` from rs-drive at the proof-verifier
// crate root so SDK consumers don't have to depend on rs-drive
// directly just to name the entry type returned by
// `verify_distinct_count_proof` and `DocumentSplitCounts::from_verified`.
pub use drive::query::SplitCountEntry;
// Same treatment for the ranked surface's entry types, plus the
// fixed-point scale the Avg axis sorts by. `RANKED_AVG_SCALE` is
// itself a re-export of grovedb's `AVG_FIXED_POINT_SCALE` — it moved
// from 10^15 to 10^19 late in grovedb's development, so clients must
// read it from here and never hardcode the literal. Divide an
// `AvgFixedPoint` value by it (or call `RankedEntryValue::as_f64`) to
// render an average.
//
// The fixed point is the exact integer grovedb ranks on **when it came
// from a proof**. `DocumentRankedEntries::from_unproved_response`
// reconstructs it from the wire's `double` (the no-proof path carries
// an f64 approximation, since a proof-verifying client rebuilds the
// entry from the proof instead), so on that path the low digits past
// f64's ~15–16 significant decimals are noise. Anything that needs the
// committed integer must go through the proof.
// `RankedPage` rides along because it is what
// `verify_ranked_top_k_proof` returns: a caller verifying a ranked
// proof for themselves needs to name the type without depending on
// rs-drive.
pub use drive::query::{RankedEntry, RankedEntryValue, RankedPage, RANKED_AVG_SCALE};
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
