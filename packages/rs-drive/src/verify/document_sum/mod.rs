//! Verifies grovedb proofs produced by the `GetDocumentsSum` endpoint.
//!
//! Mirror of [`crate::verify::document_count`] for the sum surface.
//! Pure grovedb-level verifiers as methods on
//! [`crate::query::DriveDocumentSumQuery`] that take raw `proof: &[u8]`
//! and return `(RootHash, T)`. The tenderdash signature composition
//! layer that wraps these calls lives in
//! `packages/rs-drive-proof-verifier/src/proof/document_sum.rs`.
//!
//! Carrier-aggregate verifier bodies call
//! `GroveDb::verify_aggregate_sum_query_per_key` and
//! `GroveDb::verify_aggregate_count_and_sum_query_per_key` (grovedb
//! PR #670 head `e69df59f`).

/// Carrier-aggregate-sum proof verification — sum-side analog of
/// count's `verify_carrier_aggregate_count_proof`. Returns one
/// `(in_key, i64)` per resolved In branch.
pub mod verify_carrier_aggregate_sum_proof;

/// Combined PCPS carrier-aggregate proof verification — returns one
/// `(in_key, u64 count, i64 sum)` triple per resolved In branch.
/// PCPS-only (the terminator's value tree must be a
/// `ProvableCountProvableSumTree`).
pub mod verify_carrier_aggregate_count_and_sum_proof;

/// Leaf-PCPS `AggregateCountAndSumOnRange` proof verification —
/// returns `(root_hash, u64 count, i64 sum)`. PCPS-only (the
/// terminator's value tree must be a
/// `ProvableCountProvableSumTree`). The load-bearing primitive for
/// average-range queries: the client computes
/// `avg = sum / count` locally, but the proof commits both metrics
/// from the same in-range set in one root-hash-attested traversal.
pub mod verify_aggregate_count_and_sum_proof;
