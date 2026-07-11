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
//! PR #670 head `e98bab5f`).

/// Single-aggregate-sum proof verification — sum analog of count's
/// `verify_aggregate_count_proof`. Returns `(root_hash, i64 sum)`
/// from one `AggregateSumOnRange` merk traversal.
pub mod verify_aggregate_sum_proof;

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

/// Direct read of the document type's primary-key `SumTree` element
/// — sum analog of count's `verify_primary_key_count_tree_proof`.
/// Returns `(root_hash, i64 sum)`. Used by the `documents_summable`
/// fast path on empty-where SUM queries.
pub mod verify_primary_key_sum_tree_proof;

/// Direct read of the document type's primary-key count-sum-bearing
/// element (CountSumTree / ProvableCountSumTree /
/// ProvableCountProvableSumTree) — returns `(root_hash, u64 count,
/// i64 sum)`. Used by the `documentsCountable + documentsSummable`
/// fast path on empty-where AVG queries.
pub mod verify_primary_key_count_sum_tree_proof;

/// Point-lookup sum proof verification — sum analog of count's
/// `verify_point_lookup_count_proof`. Returns one `SumEntry` per
/// verified branch (Equal-only fully-covered: one entry with empty
/// `key`; In-bearing: one entry per present In value with `key =
/// serialized_in_value`). Absent branches are silently omitted
/// because today's path query does not request absence proofs.
pub mod verify_point_lookup_sum_proof;

/// Per-distinct-key range-sum proof verification — sum analog of
/// count's `verify_distinct_count_proof`. Walks the verified
/// terminator SumTree elements and extracts each
/// `sum_value_or_default()` as a per-`(in_key, key)` entry. Used
/// by the prove path's `RangeDistinctProof` mode.
pub mod verify_distinct_sum_proof;

/// Point-lookup count+sum proof verification — AVG analog of
/// `verify_point_lookup_sum_proof`. Extracts
/// `count_sum_value_or_default()` from each verified terminator
/// element. Used by the prove path's AVG point-lookup shape on a
/// `documentsCountable + documentsSummable` doctype.
pub mod verify_point_lookup_count_and_sum_proof;

/// Per-distinct-key range-AVG proof verification — AVG analog of
/// `verify_distinct_sum_proof`. Walks the verified terminator
/// count-sum-bearing elements and extracts each
/// `count_sum_value_or_default()` as a per-`(in_key, key)`
/// `AverageEntry`. Requires the index to declare BOTH
/// `rangeCountable: true` AND `rangeSummable: true` (i.e. a
/// `rangeAverageable: true` index). Used by the prove path's
/// `RangeDistinctProof` mode on the AVG surface.
pub mod verify_distinct_count_and_sum_proof;
