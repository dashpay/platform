//! Types and module structure for the **ranked** (top-k / bottom-k)
//! document query — `SELECT <agg> GROUP BY <prop> HAVING <agg> TOP(n)`.
//!
//! A ranked query answers "which `n` groups score highest (or lowest) on
//! an aggregate?" in `O(log n + k)` with a proof, by reading grovedb's
//! per-axis *secondary* Merk of an indexed tree (grovedb PR #657). The
//! contract opts in per index via `rankedCountable` / `rankedSummable` /
//! `rankedAverageable` (meta schema v3 / PV14); the write path keeps the
//! secondaries in sync. See
//! [`crate::drive::document::ranked_index_tree_type`] for the storage
//! layout this query reads.
//!
//! The implementation is split across siblings, mirroring
//! [`super::drive_document_count_query`]:
//! - [`mode_detection`] — request-shape validation + the versioned
//!   [`mode_detection::detect_ranked_mode`] that resolves
//!   `(select, group_by, having)` into a [`DocumentRankedMode`].
//! - [`index_picker`] — [`index_picker::find_ranked_index_for_axis`],
//!   the covering-index picker for a `(group_by property, axis,
//!   aggregate field)` triple.
//! - [`path`] — the load-bearing prover/verifier-agreement path builder
//!   ([`DriveDocumentRankedQuery::indexed_property_name_tree_path`]).
//! - [`execute_top_k`] — the two executors on
//!   [`DriveDocumentRankedQuery`] (no-proof read, proof generation).
//! - [`executors`] — the `impl Drive` wrappers the dispatcher calls.
//! - [`drive_dispatcher`] — [`DocumentRankedRequest`] /
//!   [`DocumentRankedResponse`] and
//!   [`crate::drive::Drive::execute_document_ranked_request`].
//! - [`tests`] (cfg `server` + `test`) — unit + integration tests.
//!
//! ## What makes this query shape different
//!
//! Every other aggregate query in this crate walks *value trees* under a
//! property-name tree and aggregates what it finds. A ranked query never
//! touches the value trees at all: the answer lives pre-sorted in the
//! secondary Merk, keyed by `(sort_key ‖ group_key)`. Three consequences
//! shape the API:
//!
//! 1. **No `where` clauses.** Ranked indexes are single-property only
//!    (compound ones are rejected at contract-parse time in rs-dpp —
//!    their terminal level is created lazily by the same batch that
//!    populates it, which grovedb rejects for indexed trees). With one
//!    property there is no equality prefix to narrow, and a `where` on
//!    the ranked property itself would ask for a *filtered* ranking,
//!    which the secondary cannot express — it is sorted by aggregate,
//!    not by group key. Non-empty `where` is therefore rejected rather
//!    than silently ignored.
//! 2. **No `limit` / `offset` / `start_at`.** The result size is `n`,
//!    which comes from the `HAVING … TOP(n)` ranking operand. A second,
//!    independent limit could only disagree with it. Offset-paginated
//!    ranking is a separate grovedb primitive
//!    (`prove_indexed_axis_top_k_paginated`) deliberately deferred.
//! 3. **Entry order IS the ranking order.** The executor returns entries
//!    in the order grovedb walked the secondary; callers must not
//!    re-sort. Ties are broken by group key — see
//!    [`DriveDocumentRankedQuery::descending`].

#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::{DocumentTypeRef, Index};

/// The fixed-point scale grovedb's Avg axis sorts by:
/// `avg_fixed_point = floor(sum * RANKED_AVG_SCALE / count)` with
/// euclidean (toward -∞) division.
///
/// Re-exported from grovedb rather than re-declared so the two can never
/// drift — the encoded sort keys in storage are produced with grovedb's
/// constant, and a platform-side copy that fell out of step would silently
/// mis-scale every average the client renders.
#[cfg(any(feature = "server", feature = "verify"))]
pub use grovedb::element::indexed::AVG_FIXED_POINT_SCALE as RANKED_AVG_SCALE;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod index_picker;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod mode_detection;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod path;

// Server-side execution paths.
#[cfg(feature = "server")]
pub mod drive_dispatcher;
#[cfg(feature = "server")]
pub mod execute_top_k;
#[cfg(feature = "server")]
pub mod executors;

#[cfg(feature = "server")]
pub use drive_dispatcher::{DocumentRankedRequest, DocumentRankedResponse};

#[cfg(all(feature = "server", test))]
mod tests;

/// Hard ceiling on `k` (the `n` of `TOP(n)` / `BOTTOM(n)`).
///
/// The ranked proof commits one secondary entry per returned group, so
/// proof bytes grow linearly in `k`. 100 keeps the worst case in the same
/// order of magnitude as the other aggregate proof surfaces (compare
/// [`super::conditions::WhereClause::in_values`]'s 100-value cap on `In`
/// fan-out) and matches the `In` bound callers already design against.
///
/// This is a **hard** ceiling, not a clamp: a request with `n > 100` is
/// rejected with
/// [`crate::error::query::QuerySyntaxError::InvalidLimit`] rather than
/// silently truncated. Truncation would be especially treacherous here
/// because `k` is echoed inside the proof envelope and re-checked by
/// [`grovedb::GroveDb::verify_indexed_axis_top_k`] — a server-side clamp
/// would produce a proof the client's own reconstruction rejects.
#[cfg(any(feature = "server", feature = "verify"))]
pub const MAX_RANKED_LIMIT: u16 = 100;

/// Which per-group aggregate the groups are ranked by.
///
/// Maps 1:1 onto [`grovedb::element::IndexAxis`], the axis tag stored in
/// an indexed tree's TLV and echoed in the proof envelope. Kept as a
/// separate drive-side type (rather than re-exporting grovedb's) so the
/// query surface's error messages and validation can talk about
/// `rankedCountable` / `rankedSummable` / `rankedAverageable` — contract
/// grammar the storage layer knows nothing about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub enum RankedAxis {
    /// Rank by the number of documents in each group. Requires the
    /// index to declare `rankedCountable`.
    Count,
    /// Rank by the running sum of the index's `summable` property across
    /// each group. Requires `rankedSummable`.
    Sum,
    /// Rank by each group's average of the index's `summable` property,
    /// as the fixed-point value described on [`RANKED_AVG_SCALE`].
    /// Requires `rankedAverageable`.
    Avg,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl From<RankedAxis> for grovedb::element::IndexAxis {
    fn from(axis: RankedAxis) -> Self {
        match axis {
            RankedAxis::Count => grovedb::element::IndexAxis::Count,
            RankedAxis::Sum => grovedb::element::IndexAxis::Sum,
            RankedAxis::Avg => grovedb::element::IndexAxis::Avg,
        }
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl RankedAxis {
    /// The contract-grammar keyword an index must declare to be rankable
    /// on this axis. Used in error messages so a rejected query names the
    /// exact schema key the contract author has to add.
    pub fn required_index_keyword(self) -> &'static str {
        match self {
            RankedAxis::Count => "rankedCountable",
            RankedAxis::Sum => "rankedSummable",
            RankedAxis::Avg => "rankedAverageable",
        }
    }
}

/// The aggregate value carried by one ranked entry. Mirrors grovedb's
/// [`grovedb::operations::proof::indexed_axis::AxisEntries`] variants
/// exactly, one scalar at a time, so a `Vec<RankedEntry>` and an
/// `AxisEntries` carry the same information with the same types.
///
/// The variant is redundant with the request's [`RankedAxis`] by
/// construction; carrying it per entry means a decoded response is
/// self-describing (no need to thread the request alongside it to know
/// how to interpret the number), and lets both the executor and the
/// verifier fail loudly if grovedb ever hands back an axis's entries
/// under a different axis's request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub enum RankedEntryValue {
    /// Document count in the group ([`RankedAxis::Count`]).
    Count(u64),
    /// Running sum over the group ([`RankedAxis::Sum`]).
    Sum(i64),
    /// Fixed-point average over the group ([`RankedAxis::Avg`]);
    /// divide by [`RANKED_AVG_SCALE`] for the real value, or use
    /// [`Self::as_f64`].
    AvgFixedPoint(i128),
}

#[cfg(any(feature = "server", feature = "verify"))]
impl RankedEntryValue {
    /// The axis this value came from.
    pub fn axis(self) -> RankedAxis {
        match self {
            RankedEntryValue::Count(_) => RankedAxis::Count,
            RankedEntryValue::Sum(_) => RankedAxis::Sum,
            RankedEntryValue::AvgFixedPoint(_) => RankedAxis::Avg,
        }
    }

    /// The value as an `f64`, with the Avg variant scaled down by
    /// [`RANKED_AVG_SCALE`].
    ///
    /// Lossy for large counts / sums (beyond 2^53) and for averages —
    /// this is a display helper. Consensus-relevant comparisons must use
    /// the exact integer variants; two groups whose fixed-point averages
    /// differ can round to the same `f64`.
    pub fn as_f64(self) -> f64 {
        match self {
            RankedEntryValue::Count(count) => count as f64,
            RankedEntryValue::Sum(sum) => sum as f64,
            RankedEntryValue::AvgFixedPoint(avg) => (avg as f64) / (RANKED_AVG_SCALE as f64),
        }
    }
}

/// One group in a ranked result: the group's index key plus its aggregate.
///
/// `key` is the **raw index-key bytes of the grouping property's value** —
/// the same bytes that name the group's value tree under the indexed
/// property-name tree (for a `string` property, its UTF-8 bytes). Callers
/// that want the original typed value decode it with the document type's
/// key deserialization; the query layer deliberately hands back bytes so
/// prover and verifier agree without a DPP round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct RankedEntry {
    /// Raw index-key bytes of the grouped property value.
    pub key: Vec<u8>,
    /// The group's aggregate on the requested axis.
    pub value: RankedEntryValue,
}

/// A resolved ranked query. Shared by the prover and the verifier — both
/// build the grove path through
/// [`DriveDocumentRankedQuery::indexed_property_name_tree_path`], so the
/// two cannot drift on which subtree the proof is about.
///
/// Construction is normally left to
/// [`crate::drive::Drive::execute_document_ranked_request`] (server) or to
/// the SDK's proof helpers (client); both go through
/// [`index_picker::find_ranked_index_for_axis`] to resolve `index`.
#[derive(Debug, Clone)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct DriveDocumentRankedQuery<'a> {
    /// The document type being ranked.
    pub document_type: DocumentTypeRef<'a>,
    /// The contract id (32 bytes). Separate from `document_type` so the
    /// verifier can build the query without the full contract.
    pub contract_id: [u8; 32],
    /// The document type name — a path segment.
    pub document_type_name: String,
    /// The covering ranked index. Single-property by construction; its
    /// one property is both the `GROUP BY` property and the last path
    /// segment.
    pub index: &'a Index,
    /// Which aggregate the groups are ranked by. Must be covered by
    /// `index`'s matching `ranked_*` flag.
    pub axis: RankedAxis,
    /// `true` walks the secondary from the largest aggregate down
    /// (`TOP(n)` / `MAX`); `false` walks from the smallest up
    /// (`BOTTOM(n)` / `MIN`).
    ///
    /// **Tie ordering.** The secondary's keys are `(sort_key ‖
    /// group_key)`, and the walk is a plain directional scan of that
    /// keyspace — so groups with equal aggregates come back in group-key
    /// order *in the direction of the walk*: ascending group key when
    /// `descending == false`, and **descending group key when
    /// `descending == true`**. The reversal is a property of the scan,
    /// not a separate tie-break rule; it is pinned by the
    /// `ties_break_by_group_key_in_the_walk_direction` test.
    pub descending: bool,
    /// How many groups to return. `1 ..= MAX_RANKED_LIMIT`, validated in
    /// [`mode_detection`]. Fewer entries come back when the index has
    /// fewer groups than `k`; that is not an error.
    pub k: u16,
}

/// The pagination knobs a ranked request must **not** carry, bundled so
/// the versioned validator can reject them in one place.
///
/// See the module docs for why: `n` comes from the `HAVING … TOP(n)`
/// operand, so a second, independent bound could only disagree with it,
/// and `start_at` has no meaning against a keyspace sorted by aggregate
/// rather than by document id. `has_start_at` is a bare `bool` because
/// the value is never used — only its presence is an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct RankedPaginationInputs {
    /// The request's `limit`, if it set one.
    pub limit: Option<u32>,
    /// The request's `offset`, if it set one.
    pub offset: Option<u32>,
    /// Whether the request carried a `start_at` / `start_after` cursor.
    pub has_start_at: bool,
}

/// The resolved shape of a ranked request: which axis, which direction,
/// how many groups, and the `(group property, aggregate field)` pair the
/// index picker needs.
///
/// Produced by [`mode_detection::detect_ranked_mode`] from the caller's
/// `(select, group_by, having)` triple. Parallels
/// [`super::drive_document_count_query::DocumentCountMode`] in role —
/// the versioned classification of a request — but carries data rather
/// than being a bare discriminant, because the ranked surface has exactly
/// one executor pair (no-proof / proof) and all of its variation is in
/// these four values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct DocumentRankedMode {
    /// The ranking axis, from the `SELECT` function.
    pub axis: RankedAxis,
    /// Walk direction: `TOP` / `MAX` ⇒ `true`, `BOTTOM` / `MIN` ⇒ `false`.
    pub descending: bool,
    /// Number of groups requested, `1 ..= MAX_RANKED_LIMIT`.
    pub k: u16,
    /// The single `GROUP BY` property; must be the ranked index's only
    /// property.
    pub group_by_property: String,
    /// The field the aggregate applies to. Empty for
    /// [`RankedAxis::Count`] (`COUNT(*)`); the index's `summable`
    /// property for [`RankedAxis::Sum`] / [`RankedAxis::Avg`].
    pub aggregate_field: String,
}
