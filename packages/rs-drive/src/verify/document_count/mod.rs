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
/// Carrier-aggregate-count proof verification (carrier
/// `AggregateCountOnRange` composition with outer `Keys` per
/// grovedb PR #663) — returns one `(in_key, u64)` per resolved In
/// branch. Used by `group_by = [in_field]` count queries that
/// carry both an `In` clause and a range clause.
pub mod verify_carrier_aggregate_count_proof;
/// Distinct-count proof verification (regular range proof against a
/// `ProvableCountTree`) — returns the per-`(in_key, key)` entries the
/// proof commits to.
pub mod verify_distinct_count_proof;
/// Point-lookup count proof verification (CountTree element proof for
/// Equal/`In` counts on a `countable: true` index) — returns one entry
/// per covered branch, with each `count` extracted from the verified
/// CountTree element's `count_value`.
pub mod verify_point_lookup_count_proof;
/// Primary-key CountTree proof verification — used by the
/// `documents_countable: true` fast path for unfiltered total counts
/// at the doctype level. Returns a single `u64` count.
pub mod verify_primary_key_count_tree_proof;
