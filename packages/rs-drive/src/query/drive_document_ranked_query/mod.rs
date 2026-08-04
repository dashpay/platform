//! Types and module structure for the **ranked** (top-k / bottom-k)
//! document query —
//! `SELECT <agg> GROUP BY <prop> ORDER BY <agg> DESC LIMIT n OFFSET m`.
//!
//! A ranked query answers "which `n` groups score highest (or lowest) on
//! an aggregate, starting from rank `m`?" in `O(log n + k)` with a proof,
//! by reading grovedb's per-axis *secondary* Merk of an indexed tree
//! (grovedb PR #657). The contract opts in per index via
//! `rankedCountable` / `rankedSummable` / `rankedAverageable` (meta
//! schema v3 / PV14); the write path keeps the secondaries in sync. See
//! [`crate::drive::document::ranked_index_tree_type`] for the storage
//! layout this query reads.
//!
//! The implementation is split across siblings, mirroring
//! [`super::drive_document_count_query`]:
//! - [`mode_detection`] — request-shape validation + the versioned
//!   [`mode_detection::detect_ranked_mode`] that resolves
//!   `(select, group_by, order_by, limit, offset)` into a
//!   [`DocumentRankedMode`].
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
//! 2. **`limit` is mandatory, `offset` is free, `start_at` is refused.**
//!    `limit` is the `k` of the walk and the ranked surface has no
//!    server default for it, so it must be supplied. `offset` is the
//!    rank the page starts at and is unbounded above: grovedb's
//!    paginated prover is `O(log n + k)` *regardless of offset* (the
//!    skipped region is attested by counted subtree commitments, never
//!    walked entry by entry), so a large offset is not a cost lever and
//!    needs no ceiling. `start_at` / `start_after` name a document id,
//!    which does not appear anywhere in an aggregate-ordered keyspace.
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

/// Hard ceiling on `k` (the request's `LIMIT`).
///
/// The ranked proof commits one secondary entry per returned group, so
/// proof bytes grow linearly in `k`. 100 keeps the worst case in the same
/// order of magnitude as the other aggregate proof surfaces (compare
/// [`super::conditions::WhereClause::in_values`]'s 100-value cap on `In`
/// fan-out) and matches the `In` bound callers already design against.
///
/// This is a **hard** ceiling, not a clamp: a request with `limit > 100`
/// is rejected with
/// [`crate::error::query::QuerySyntaxError::InvalidLimit`] rather than
/// silently truncated. Truncation would be especially treacherous here
/// because `k` is echoed inside the proof envelope and re-checked by
/// [`grovedb::GroveDb::verify_indexed_axis_top_k_paginated`] — a
/// server-side clamp would produce a proof the client's own
/// reconstruction rejects.
///
/// There is deliberately **no companion ceiling on `OFFSET`**; see the
/// module docs and [`DriveDocumentRankedQuery::offset`].
#[cfg(any(feature = "server", feature = "verify"))]
pub const MAX_RANKED_LIMIT: u16 = 100;

/// The `ORDER BY` field name that means "the group's `COUNT(*)`".
///
/// `COUNT(*)` has no field to name, so the ranked grammar needs some
/// token for "order by the thing the select projects". `$count` is
/// chosen because the leading `$` is DPP's **system-property
/// namespace** (`$id`, `$ownerId`, `$revision`, `$createdAt`, …): a
/// document schema cannot declare a property whose name starts with
/// `$`, so the sentinel is guaranteed not to collide with any real
/// property a contract author could write, now or in any future
/// contract. That is the whole reason it is spelled with a sigil rather
/// than as `"count"` — a bare `count` would silently hijack ordering
/// for any schema that happens to have a `count` column.
#[cfg(any(feature = "server", feature = "verify"))]
pub const RANKED_COUNT_ORDER_KEY: &str = "$count";

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
    /// (`ORDER BY <agg> DESC`); `false` walks from the smallest up
    /// (`ORDER BY <agg> ASC`).
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
    /// How many groups to return — the request's `LIMIT`.
    /// `1 ..= MAX_RANKED_LIMIT`, validated in [`mode_detection`]. Fewer
    /// entries come back when the index has fewer groups than
    /// `offset + k`; that is not an error.
    pub k: u16,
    /// How many ranks to skip before the returned page — the request's
    /// `OFFSET`. `0` for an unpaginated ranking.
    ///
    /// Unbounded above (any `u32`), on purpose. grovedb attests the
    /// skipped region through the counted subtree commitments
    /// (`HashWithCount` / `HashWithCountAndSum`) rather than by walking
    /// it, so both the prover's work and the proof's size stay
    /// `O(log n + k)` **at any offset** — an offset of 4 and an offset
    /// of four billion cost the same. There is therefore no
    /// denial-of-service lever to cap, and capping would only stop
    /// honest deep pagination.
    ///
    /// An offset past the end of the secondary is a provable answer, not
    /// an error: the page comes back empty and
    /// [`RankedPage::skipped`] is the secondary's entire population.
    pub offset: u32,
}

/// A page of a ranked result: the entries, plus how many ranks were
/// actually skipped to reach them.
///
/// `skipped` is what turns a page into a *ranking*: entry `i` of
/// `entries` is the group at rank `skipped + i` (0-based). Without it a
/// caller that asked for `OFFSET 4 LIMIT 1` would receive one entry and
/// have to trust the server that it really is the 5th-best group.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct RankedPage {
    /// Number of secondary entries skipped before this page.
    ///
    /// On the **proved** path this is grovedb's cryptographically
    /// attested count, independently re-derived by the verifier from the
    /// counted subtree commitments in the proof bytes: it equals the
    /// requested offset unless the walk ran out of entries first, in
    /// which case `entries` is empty and `skipped` is a proof that the
    /// secondary holds exactly `skipped` groups in total.
    ///
    /// On the **unproven** read there is nothing to attest and grovedb's
    /// read API does not report the short walk, so this is simply the
    /// requested offset. The two paths therefore disagree in exactly one
    /// case — an offset past the end — where the unproven read reports
    /// the requested offset and the proved one reports the true
    /// population. Callers that need the population must prove.
    pub skipped: u64,
    /// The groups on this page, **in ranking order**. Never longer than
    /// the query's `k`.
    pub entries: Vec<RankedEntry>,
}

/// The pagination knobs a ranked request carries, bundled so the
/// versioned validator reads them in one place.
///
/// `limit` is required (it is the ranking's `k`), `offset` is optional
/// and defaults to `0`, and `start_at` is refused outright — a cursor
/// names a document id, and document ids do not appear in a keyspace
/// sorted by aggregate. `has_start_at` is a bare `bool` because the
/// value is never used; only its presence is an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct RankedPaginationInputs {
    /// The request's `limit`. Required in ranked mode.
    pub limit: Option<u32>,
    /// The request's `offset`, if it set one. `None` means rank 0.
    pub offset: Option<u32>,
    /// Whether the request carried a `start_at` / `start_after` cursor.
    pub has_start_at: bool,
}

/// The resolved shape of a ranked request: which axis, which direction,
/// how many groups, and the `(group property, aggregate field)` pair the
/// index picker needs.
///
/// Produced by [`mode_detection::detect_ranked_mode`] from the caller's
/// `(select, group_by, order_by, limit, offset)` inputs. Parallels
/// [`super::drive_document_count_query::DocumentCountMode`] in role —
/// the versioned classification of a request — but carries data rather
/// than being a bare discriminant, because the ranked surface has exactly
/// one executor pair (no-proof / proof) and all of its variation is in
/// these five values.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct DocumentRankedMode {
    /// The ranking axis, from the `SELECT` function.
    pub axis: RankedAxis,
    /// Walk direction: `ORDER BY … DESC` ⇒ `true`, `ASC` ⇒ `false`.
    pub descending: bool,
    /// Number of groups requested — the `LIMIT`, `1 ..= MAX_RANKED_LIMIT`.
    pub k: u16,
    /// Ranks to skip — the `OFFSET`, `0` when unset.
    pub offset: u32,
    /// The single `GROUP BY` property; must be the ranked index's only
    /// property.
    pub group_by_property: String,
    /// The field the aggregate applies to. Empty for
    /// [`RankedAxis::Count`] (`COUNT(*)`); the index's `summable`
    /// property for [`RankedAxis::Sum`] / [`RankedAxis::Avg`].
    pub aggregate_field: String,
}
