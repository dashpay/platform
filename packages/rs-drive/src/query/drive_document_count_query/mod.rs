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
pub mod executors;

#[cfg(feature = "server")]
pub use drive_dispatcher::{DocumentCountRequest, DocumentCountResponse};
#[cfg(feature = "server")]
pub use execute_range_count::RangeCountOptions;

/// Hard cap on entries the count fan-out arms ask the executor
/// to return.
///
/// Count fan-out (`PerInValue`, and the Aggregate + range
/// sub-case of `RangeNoProof`) emits at most one entry per `In`
/// value, and `In` is structurally capped at 100 by
/// [`super::conditions::WhereClause::in_values`]. This cap sits
/// well above the real bound. Two reasons to pin it explicitly
/// instead of leaning on the operator-tunable
/// `default_query_limit`:
///
/// 1. `default_query_limit` is a documents-fetch knob — applying
///    it to count fan-out can truncate aggregate sums below |In|
///    under tighter operator tuning, silently producing wrong
///    totals.
/// 2. Pinning a number here keeps the dispatcher's correctness
///    independent of operator configuration.
///
/// `1024` is high enough that the cap never fires under the
/// current `WhereClause::in_values` policy. If a future code
/// change makes it reachable, treat that as a signal to revisit
/// the bound before raising the constant.
///
/// # Pattern: failsafe cap for structurally-bounded ops
///
/// This is the prototype of a small project convention: when an
/// executor-level operation has a structural upper bound enforced
/// upstream (here, `WhereClause::in_values()`'s 100-cap on the In
/// array), pin a failsafe cap at the executor boundary that sits
/// well above the upstream bound rather than reusing an unrelated
/// operator-tunable limit. The failsafe never fires under the
/// upstream constraint — it exists to (a) keep behavior
/// independent of operator config, and (b) localize the blast
/// radius if the upstream constraint ever loosens. Constants
/// added under this pattern should follow the
/// `MAX_<OPERATION>_AS_FAILSAFE` naming so the role is visible
/// at the use site.
#[cfg(feature = "server")]
pub const MAX_LIMIT_AS_FAILSAFE: u32 = 1024;

/// Platform-wide **maximum** outer-walk cap for carrier-aggregate
/// range-outer proofs (chapter 30 G8: `outer_range_field > X AND
/// inner_acor_field > Y` with `group_by = [outer_range_field]` and
/// `prove = true`).
///
/// The cap bounds the proof size: bytes grow linearly with the
/// number of outer matches (~1 700 B per outer key in this
/// chapter's widget fixture; `10 × 1 700 B ≈ 17 KB` worst case).
/// 10 keeps the worst-case proof comfortably inside Tier-1 of the
/// visualizer's shareable-link guidance (< 20 KB).
///
/// **Caller semantics:**
/// - `request.limit = None` → server uses `MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`
///   (the default).
/// - `request.limit = Some(n)` with `n ≤ MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`
///   → accepted; the dispatcher passes `n` through to
///   `SizedQuery::limit` so the prover walks exactly `n` outer matches.
/// - `request.limit = Some(n)` with `n > MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`
///   → rejected with `InvalidLimit`. The cap is a hard ceiling: callers
///   that want more results must call repeatedly with disjoint
///   outer-range windows.
///
/// Why the ceiling is a hardcoded compile-time constant rather
/// than `drive_config.max_query_limit` (the operator-tunable
/// runtime value): on the prove path, `SizedQuery::limit` is
/// part of the serialized `PathQuery` and feeds the merk-root
/// reconstruction. Anchoring the ceiling to a compile-time
/// constant guarantees prover and verifier agree on what the
/// "default when None" value is, regardless of operator config
/// (same rationale as `RangeDistinctProof`'s use of
/// `crate::config::DEFAULT_QUERY_LIMIT`).
pub const MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT: u16 = 10;

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
    ///
    /// Three-valued by design:
    /// - `Some(n)` with `n > 0` — verified count for an entry the
    ///   underlying data path materialized.
    /// - `Some(0)` — caller queried this branch and the executor
    ///   confirmed zero matching documents. Emitted by the no-proof
    ///   point-lookup path's aggregated total wrapper (a single
    ///   summed entry whose value can be 0) and by the no-proof range
    ///   executors when their walk returns nothing. Not emitted
    ///   per-In-branch under the current shape — see `None` below.
    /// - `None` — reserved for a future absence-proof variant. The
    ///   current `point_lookup_count_path_query` doesn't set
    ///   `absence_proofs_for_non_existing_searched_keys: true`, so
    ///   absent In branches are **omitted from the verified entry
    ///   list entirely** (grovedb's `verify_query` doesn't surface
    ///   `(path, key, None)` triples for them). Callers that need to
    ///   distinguish "queried but absent" diff the request's In array
    ///   against the returned entries by key. The variant exists in
    ///   the type signature so a future path-query change that flips
    ///   the flag surfaces absences via `count: None` without a
    ///   breaking struct change — distinguishable from `Some(0)`
    ///   (which a zero-count CountTree could never produce on its own
    ///   since zero-count CountTrees aren't materialized in merk).
    pub count: Option<u64>,
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
/// **Result shape vs. executor strategy.** Each variant names a
/// result shape — the per-variant docstring lists the
/// where-clause shapes that route to that result shape and
/// notes which executor strategy
/// [`DriveDocumentCountQuery::detect_mode`] picks for each.
/// `(in_field, range_field)` combinations on the same request
/// are accepted on multiple `CountMode` variants — the executor
/// strategy distinguishes them. Upstream routing
/// (drive-abci's `validate_and_route`) picks the `CountMode`
/// from the caller's `group_by`; downstream `detect_mode`
/// converts the `(CountMode, where_clauses, prove)` triple into
/// the resolved [`DocumentCountMode`].
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
    /// Where-clause shapes accepted:
    /// - one `In` clause on `group_by[0]` (no range clause): the
    ///   canonical shape — routes to `PointLookupProof` on the
    ///   prove path, `PerInValue` on the no-proof path.
    /// - one `In` on `group_by[0]` AND a range clause on a
    ///   different field: routes to
    ///   `RangeAggregateCarrierProof` on the prove path
    ///   (grovedb #663 carrier-ACOR — one verified `u64` per
    ///   In branch, range collapsed) and `RangeNoProof` on the
    ///   no-prove path (per-In-branch range walk). Both produce
    ///   entries that line up with the caller's GROUP BY shape.
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
    /// Where-clause shapes accepted:
    /// - one range clause on `group_by[0]` (no `In` clause):
    ///   canonical RangeDistinctProof / RangeNoProof distinct.
    /// - one range on `group_by[0]` AND an `In` clause on a
    ///   different field: prove path keeps `RangeDistinctProof`
    ///   with In-fanout via grovedb subquery; no-prove path uses
    ///   `RangeNoProof` distinct on the merged result. Per-
    ///   distinct-value entries cover both branches of the In.
    /// - two range clauses on different fields, the second
    ///   being `group_by[0]`: routes to
    ///   `RangeAggregateCarrierProof` (outer range + inner-ACOR
    ///   carrier per grovedb #664 outer-range cap). See
    ///   `outer_range_plus_inner_range_with_prove_and_group_by_range_routes_to_carrier_proof`
    ///   for the regression test pinning this shape.
    ///
    /// `limit` caps the number of distinct values; on the prove
    /// path it's validated-not-clamped (oversized values rejected
    /// with `InvalidLimit`).
    GroupByRange,

    /// `select=COUNT, group_by=[in_field, range_field]`. One entry
    /// per `(in_key, range_key)` pair.
    ///
    /// Where-clause invariants: an `In` clause on `group_by[0]`
    /// AND a range clause on `group_by[1]` (match-any over
    /// the where-clauses list — clause ordering on the wire
    /// doesn't affect routing).
    /// `limit` is a **global cap on the emitted `(in_key, key)` lex
    /// stream**, not per-In-branch. The executor pushes a single
    /// `SizedQuery::limit` over the compound walk, so a request
    /// with `|In| = 3` and `limit = 5` returns at most 5 entries
    /// total across all In branches (ordered by `(in_key, key)`,
    /// direction from the first `order_by` clause). On the prove
    /// path it's validated-not-clamped (oversized values rejected
    /// with `InvalidLimit`).
    GroupByCompound,
}

impl CountMode {
    /// `true` for [`Self::Aggregate`] (single-row response);
    /// `false` for the three grouped variants. See each variant's
    /// docstring for the per-shape semantics.
    pub fn is_aggregate(self) -> bool {
        matches!(self, Self::Aggregate)
    }

    /// `true` for [`Self::GroupByRange`] and [`Self::GroupByCompound`]
    /// — the two variants whose proof shape requires per-distinct-
    /// value `KVCount` ops. See each variant's docstring for the
    /// per-shape proof routing.
    pub fn requires_distinct_walk(self) -> bool {
        matches!(self, Self::GroupByRange | Self::GroupByCompound)
    }

    /// `true` for [`Self::GroupByRange`] and [`Self::GroupByCompound`]
    /// — the two variants whose result size isn't structurally
    /// bounded. [`Self::Aggregate`] and [`Self::GroupByIn`] reject
    /// `limit` upstream; see each variant's docstring for the
    /// per-shape reasoning.
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
    /// Exactly one `In` clause + one range clause + `prove = true`
    /// + [`CountMode::GroupByIn`] — produces a grovedb carrier
    /// `AggregateCountOnRange` proof: one outer-key descent per
    /// `In` value, each terminating in an ACOR boundary walk over
    /// the per-branch range subtree. Returns one `(in_key, u64)`
    /// pair per resolved In branch — same per-key aggregate
    /// semantics as the no-proof per-In fan-out, just verifiable.
    ///
    /// Proof size is `O(|In values| · (log B + log C'))` where `B`
    /// is the In-property's distinct-value count and `C'` is the
    /// terminator subtree's distinct-value count. Smaller than the
    /// alternative [`Self::RangeDistinctProof`] (which scales with
    /// the number of distinct in-range terminator values per
    /// branch, not per-branch log-bound boundary nodes) and
    /// preserves per-In aggregate granularity that GROUP BY
    /// `[in_field, range_field]` can't express.
    ///
    /// Path-query shape (see
    /// [`DriveDocumentCountQuery::carrier_aggregate_count_path_query`]):
    /// outer Keys = serialized In values; subquery_path = ranged
    /// property name; subquery = ACOR(range). Verified via
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`]
    /// (returns `Vec<(Vec<u8>, u64)>`).
    ///
    /// Enabled by grovedb PR #663 ("allow AggregateCountOnRange as
    /// carrier subquery"). Before that PR this shape was rejected
    /// in [`Self::detect_mode`] with the message "range count
    /// queries with an `in` clause are not supported on the
    /// aggregate prove path".
    RangeAggregateCarrierProof,
}
