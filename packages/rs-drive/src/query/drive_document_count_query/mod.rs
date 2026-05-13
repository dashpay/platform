//! Types and module structure for the `GetDocumentsCount` query.
//!
//! The implementation is split across siblings:
//! - [`mode_detection`] — operator classification + `detect_mode`.
//! - [`index_picker`] — covering-index pickers
//!   (`find_countable_index_*`, `find_range_countable_index_*`).
//! - [`path_query`] — the load-bearing prover/verifier-agreement
//!   path-query builders (`aggregate_count_path_query`,
//!   `distinct_count_path_query`, `range_clause_to_query_item`).
//! - [`execute_point_lookup`] — Equal/In point-lookup execution
//!   (`execute_no_proof`, `execute_with_proof`).
//! - [`execute_range_count`] — range-mode execution + `RangeCountOptions`.
//! - [`drive_dispatcher`] — `impl Drive` per-mode dispatchers +
//!   `DocumentCountRequest` / `DocumentCountResponse` +
//!   `execute_document_count_request`.
//! - [`tests`] (cfg `server` + `test`) — integration tests.
//!
//! This file owns the three public types every other submodule
//! references and the corresponding `mod` / `pub use` plumbing.

use dpp::data_contract::document_type::{DocumentTypeRef, Index};

use super::conditions::WhereClause;

// Re-exports for the submodules and the `tests` module's
// `use super::*;`. `WhereOperator` is used by every submodule that
// builds path queries or executes; `QuerySyntaxError` is the canonical
// error variant the mode detector and dispatchers surface.
#[cfg(any(feature = "server", feature = "verify"))]
pub use super::conditions::WhereOperator;
#[cfg(any(feature = "server", feature = "verify"))]
pub use crate::error::query::QuerySyntaxError;

pub mod mode_detection;
// Index pickers + path-query builders are reachable from both the
// server prove path and the SDK proof verifier; their submodule cfgs
// match.
pub mod index_picker;
pub mod path_query;

// Server-side execution paths.
#[cfg(feature = "server")]
pub mod drive_dispatcher;
#[cfg(feature = "server")]
pub mod execute_point_lookup;
#[cfg(feature = "server")]
pub mod execute_range_count;

#[cfg(feature = "server")]
pub use drive_dispatcher::{DocumentCountRequest, DocumentCountResponse};
#[cfg(feature = "server")]
pub use execute_range_count::RangeCountOptions;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests;

/// A query to count documents using CountTree elements in the index path.
///
/// This struct encapsulates all the information needed to perform a count
/// query on a document type's countable index.
#[derive(Debug, Clone)]
pub struct DriveDocumentCountQuery<'a> {
    /// The document type to count
    pub document_type: DocumentTypeRef<'a>,
    /// The contract id (32 bytes)
    pub contract_id: [u8; 32],
    /// The document type name
    pub document_type_name: String,
    /// The countable index to use
    pub index: &'a Index,
    /// The equality where clauses that match index prefix properties
    pub where_clauses: Vec<WhereClause>,
}

/// An entry in a split count result, containing the serialized
/// key(s) and the count of documents matching them.
///
/// For flat queries (per-`In`-value mode without a range, or
/// per-distinct-value-in-range mode without an `In` on prefix) only
/// `key` is meaningful and `in_key` is `None`.
///
/// For compound range-distinct queries (an `In` clause on a prefix
/// property plus a range on the terminator) BOTH keys are carried:
/// `in_key` is the In-fork's prefix value and `key` is the
/// terminator value. Cross-fork aggregation is intentionally NOT
/// done server-side — emitting the unmerged per-(in_key, key) shape
/// lets `limit` push directly into grovedb (no pre-merge issue),
/// keeps proof verification straightforward (no absence-proof
/// gymnastics for omitted In branches), and gives callers strictly
/// more information than a flat histogram. Callers reduce
/// client-side when they want the sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitCountEntry {
    /// The serialized prefix key for compound queries (the `In`
    /// value for this fork). `None` for flat queries.
    pub in_key: Option<Vec<u8>>,
    /// The serialized terminator/value key for this entry.
    pub key: Vec<u8>,
    /// The count of documents matching this `(in_key, key)` tuple
    /// (or just `key` for flat queries).
    pub count: u64,
}

/// SQL-shaped count-query mode — names the response shape the
/// caller asked for via `(select, group_by)` on the wire.
///
/// **Two count-mode enums coexist in this module.** This one names
/// the *output shape* the request produces (single aggregate vs
/// per-group entries). [`DocumentCountMode`] below names the
/// *executor strategy* (which proof primitive / which walk path
/// Drive uses to compute that shape). `CountMode` lives on
/// [`DocumentCountRequest`] as the caller-supplied contract;
/// `DocumentCountMode` is derived from `(CountMode, where_clauses,
/// prove)` by [`DriveDocumentCountQuery::detect_mode`] just before
/// dispatch.
///
/// The invariants below are enforced upstream (in drive-abci's
/// `validate_and_route`) before a `DocumentCountRequest` is built.
/// They're documented here so any new caller knows the
/// shape-validity contract attached to each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountMode {
    /// `select=COUNT, group_by=[]`. Single u64 result.
    ///
    /// Where-clause shapes accepted:
    /// - empty (relies on a `documentsCountable: true` doctype),
    /// - Equal-only (fully covered by a `countable: true` index),
    /// - one `In` (per-In fan-out, summed server-side),
    /// - one range (uses `AggregateCountOnRange` for prove,
    ///   `RangeNoProof` for no-proof),
    /// - one `In` + one range on the no-proof path (per-In fan-out
    ///   each doing a range walk; prove is rejected).
    ///
    /// `limit` is structurally meaningless (aggregate is one row)
    /// and is rejected upstream when set.
    Aggregate,

    /// `select=COUNT, group_by=[in_field]`. One entry per `In` value.
    ///
    /// Where-clause invariants: exactly one `In` clause whose field
    /// matches `group_by[0]`; no range clause.
    ///
    /// `limit` is rejected upstream when set. The In array is
    /// already capped at 100 entries by `WhereClause::in_values()`,
    /// so the result size is bounded by construction; a separate
    /// `limit` would either be redundant (≤ 100) or would silently
    /// truncate the proof to fewer In branches than the caller
    /// asked for (because the PointLookupProof path can't represent
    /// a partial-In-array selection in its `SizedQuery`). Callers
    /// that want fewer branches narrow the In array directly.
    GroupByIn,

    /// `select=COUNT, group_by=[range_field]`. One entry per distinct
    /// value within the range.
    ///
    /// Where-clause invariants: exactly one range clause whose field
    /// matches `group_by[0]`; no `In` clause.
    /// `limit` caps the number of distinct values; on the prove
    /// path it's validated-not-clamped (oversized values rejected
    /// with `InvalidLimit`).
    GroupByRange,

    /// `select=COUNT, group_by=[in_field, range_field]`. One entry
    /// per `(in_key, range_key)` pair.
    ///
    /// Where-clause invariants: exactly one `In` clause on `group_by[0]`
    /// AND exactly one range clause on `group_by[1]`.
    /// `limit` caps entries *per In branch* (not globally).
    GroupByCompound,
}

impl CountMode {
    /// Whether this mode produces a single aggregate u64 (vs
    /// per-group entries). Aggregate is the `select=COUNT,
    /// group_by=[]` shape; the three grouped variants produce
    /// entries.
    pub fn is_aggregate(self) -> bool {
        matches!(self, Self::Aggregate)
    }

    /// Whether this mode requires the distinct walk on a range
    /// clause (per-distinct-value entries via `KVCount` ops). Only
    /// the two range-grouped variants do; aggregate and per-In
    /// take other paths even when a range clause is present.
    pub fn requires_distinct_walk(self) -> bool {
        matches!(self, Self::GroupByRange | Self::GroupByCompound)
    }

    /// Whether the caller-supplied `limit` is meaningful for this
    /// mode. Two variants accept it:
    ///
    /// - [`Self::GroupByRange`] — bounds the number of distinct
    ///   range values returned (the range itself is unbounded).
    /// - [`Self::GroupByCompound`] — same, applied per In branch.
    ///
    /// The other two variants reject `limit` upstream:
    ///
    /// - [`Self::Aggregate`] — result is a single row.
    /// - [`Self::GroupByIn`] — result is bounded by the In array
    ///   (capped at 100 entries by `WhereClause::in_values()`);
    ///   silent partial truncation isn't representable on the
    ///   PointLookupProof path, so the limit would be misleading
    ///   either way.
    pub fn accepts_limit(self) -> bool {
        matches!(self, Self::GroupByRange | Self::GroupByCompound)
    }
}

/// Classification of a count query's shape, used to dispatch to the
/// right executor. Returned by
/// [`DriveDocumentCountQuery::detect_mode`].
///
/// The discriminator is purely a function of the where-clause
/// operators + the caller's [`CountMode`] + `prove`; it does not
/// depend on the contract's index set. Picking a covering index for
/// the chosen mode is a separate step that requires the document
/// type's `BTreeMap<String, Index>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentCountMode {
    /// No range, no `In` — single summed entry with empty key. Reads
    /// the `CountTree` count directly at the indexed path.
    Total,
    /// Exactly one `In` clause, no range — one entry per (deduped)
    /// `In` value, each computed as the count at that single value.
    /// The `In` doubles as the per-value split signal.
    PerInValue,
    /// Exactly one range clause, no proof — walks the property-name
    /// `ProvableCountTree`'s children inside the range. Returns either
    /// a single summed entry or per-distinct-value entries depending
    /// on whether the caller's [`CountMode`] requires a distinct walk
    /// ([`CountMode::GroupByRange`] / [`CountMode::GroupByCompound`])
    /// or not ([`CountMode::Aggregate`]).
    RangeNoProof,
    /// Exactly one range clause + `prove = true` +
    /// [`CountMode::Aggregate`] — produces a grovedb
    /// `AggregateCountOnRange` proof that verifies to a single u64.
    /// The merk-level primitive returns one aggregate; per-distinct-
    /// value entries with proof go through [`Self::RangeDistinctProof`]
    /// instead.
    RangeProof,
    /// Exactly one range clause + `prove = true` +
    /// [`CountMode::GroupByRange`] or [`CountMode::GroupByCompound`]
    /// — produces a regular range proof against the property-name
    /// `ProvableCountTree`. The
    /// proof's `KVCount(key, value, count)` ops carry per-distinct-
    /// value counts, each cryptographically committed via
    /// `node_hash_with_count` to the merk root. The verifier walks the
    /// proof op stream and emits a per-key count map, no opt-in
    /// aggregate-collapse wrapper. Proof size is O(distinct values
    /// matched) rather than the O(log n) of [`Self::RangeProof`], but
    /// still much smaller than materialize-and-count.
    RangeDistinctProof,
    /// No range clause + `prove = true` — produces a per-branch
    /// `Element::CountTree` proof. Either an unfiltered total
    /// (`documents_countable: true` fast path, proving the
    /// doctype's primary-key CountTree directly) or a covered
    /// Equal/`In` lookup against a `countable: true` index (proving
    /// one CountTree element per matched branch via
    /// [`DriveDocumentCountQuery::point_lookup_count_path_query`]).
    /// Proof size is O(k × log n) where k is the number of covered
    /// branches (1 for the empty-where fast path and Equal-only
    /// fully-covered case; ≤ |In values| for In-on-prefix). No
    /// document materialization, no `u16::MAX` matching-docs cap —
    /// the merk-level `count_value` IS the result, the SDK
    /// extracts it via `verify_point_lookup_count_proof`.
    PointLookupProof,
}
