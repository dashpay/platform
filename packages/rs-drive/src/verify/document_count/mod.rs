//! Verifies grovedb proofs produced by the `GetDocumentsCount` endpoint.
//!
//! Mirrors the layering used by `packages/rs-drive/src/verify/document/`:
//! pure grovedb-level verifiers as methods on
//! [`DriveDocumentCountQuery`](crate::query::DriveDocumentCountQuery)
//! that take raw `proof: &[u8]` and return `(RootHash, T)`. The tenderdash
//! signature composition layer that wraps these calls lives in
//! `packages/rs-drive-proof-verifier/src/proof/document_count.rs`.

/// Aggregate-count proof verification (`AggregateCountOnRange`
/// primitive) — returns a single `u64`.
pub mod verify_aggregate_count_proof;
/// Distinct-count proof verification (regular range proof against a
/// `ProvableCountTree`) — returns the per-`(in_key, key)` entries the
/// proof commits to.
pub mod verify_distinct_count_proof;
