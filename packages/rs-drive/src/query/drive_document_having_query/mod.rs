//! Types and module structure for the **boolean-`HAVING` range** document
//! query — `SELECT <agg> GROUP BY <prop> HAVING <agg> <op> <value>
//! [ORDER BY <agg> ASC|DESC] LIMIT n`.
//!
//! A having-range query answers "which groups' aggregate falls inside a
//! value bound?" ("hashtags with more than 100 posts") in `O(log n + k)`
//! with a proof, by range-reading the same per-axis *secondary* Merk the
//! ranked query walks (grovedb PR #657): the secondary is keyed by
//! `(sort_key ‖ group_key)` with an order-preserving sort-key encoding,
//! so an inclusive numeric bound on the aggregate is a contiguous byte
//! range in the secondary's keyspace. The same contract opt-in applies —
//! `rankedCountable` / `rankedSummable` / `rankedAverageable` (meta schema
//! v3 / PV14) — and a `HAVING` on an axis the index does not declare is
//! rejected, because serving it would mean walking every group.
//!
//! The implementation mirrors [`super::drive_document_ranked_query`]
//! sibling-for-sibling and reuses its axis / entry / pagination types
//! ([`RankedAxis`], [`RankedEntry`], [`RankedEntryValue`],
//! [`super::RankedPaginationInputs`]) and its covering-index picker —
//! both surfaces read the same tree, so sharing the resolution logic is
//! what keeps them provably about the same subtree:
//! - [`mode_detection`] — request-shape validation + the versioned
//!   `(select, group_by, having, order_by, limit)` →
//!   [`DocumentHavingMode`] resolution, including the operator →
//!   inclusive-bounds translation.
//! - [`execute_range`] — the two executors on
//!   [`DriveDocumentHavingQuery`] (no-proof read, proof generation).
//! - [`executors`] — the `impl Drive` wrappers the dispatcher calls.
//! - [`drive_dispatcher`] — [`DocumentHavingRequest`] /
//!   [`DocumentHavingResponse`] and
//!   [`crate::drive::Drive::execute_document_having_request`].
//! - [`tests`] (cfg `server` + `test`) — unit + integration tests.
//!
//! ## What makes this query shape different from ranked
//!
//! Ranked addresses groups by **rank position** (`k` best, starting at
//! rank `offset`); having-range addresses them by **value bound**
//! (`aggregate ∈ [lo, hi]`). Three consequences:
//!
//! 1. **The bound is part of the proof contract.** The grovedb envelope
//!    for a range read echoes the Merk query itself, and the verifier
//!    re-builds that query from the request's bounds
//!    ([`AxisRangeBounds::merk_query`]) — so prover and verifier must
//!    share one bounds-to-query translation, exactly as they share the
//!    grove path. Completeness comes from the Merk range proof: the
//!    boundary commitments show no in-range group was omitted.
//! 2. **No `OFFSET`, no `start_at` — and no full pagination.** The
//!    range primitives take a limit but no skip, and a request carrying
//!    either knob is rejected loudly. A page cut at `limit` can only be
//!    continued past **distinct** aggregate values, by tightening the
//!    bound past the last value seen; a cut that lands **inside a tie**
//!    (several groups sharing the boundary aggregate) cannot be
//!    continued at all — moving the threshold past the tied value skips
//!    the uncollected tied groups, and keeping it returns the same
//!    page. Enumerating through a tie wider than [`MAX_HAVING_LIMIT`]
//!    needs a cursor on the `(sort_key ‖ group_key)` composite
//!    keyspace, a future capability; until then, size `limit` above the
//!    widest tie the data can produce, or accept the cut.
//! 3. **Entry order is axis order in the walk direction.** Ascending by
//!    default (`ORDER BY` is optional here — the bound, not the
//!    ordering, is the point of the query); an explicit `ORDER BY` on
//!    the selected aggregate flips the walk. Ties break by group key in
//!    the direction of the walk, same as ranked.

#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::version::PlatformVersion;
#[cfg(any(feature = "server", feature = "verify"))]
use std::collections::BTreeMap;

#[cfg(any(feature = "server", feature = "verify"))]
use super::drive_document_ranked_query::index_picker::{
    encode_prefix_branches, find_ranked_index_for_axis, no_covering_index_message,
};
#[cfg(any(feature = "server", feature = "verify"))]
use super::drive_document_ranked_query::{
    path::indexed_property_name_tree_path_for_index, PrefixPin, RankedAxis,
};
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::drive::DriveError;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::query::QuerySyntaxError;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::Error;
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::element::indexed::{encode_avg_sort_key, encode_count_sort_key, encode_sum_sort_key};
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::Query;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod mode_detection;

// Server-side execution paths.
#[cfg(feature = "server")]
pub mod drive_dispatcher;
#[cfg(feature = "server")]
pub mod execute_range;
#[cfg(feature = "server")]
pub mod executors;

#[cfg(feature = "server")]
pub use drive_dispatcher::{DocumentHavingRequest, DocumentHavingResponse};

#[cfg(all(feature = "server", test))]
mod tests;

/// Hard ceiling on a having-range request's `LIMIT`. Same value and same
/// rationale as [`super::drive_document_ranked_query::MAX_RANKED_LIMIT`]:
/// the proof commits one secondary entry per returned group, so proof
/// bytes grow linearly in the limit, and the ceiling is a hard rejection
/// rather than a clamp because the limit is echoed in the proof envelope
/// and re-checked by the verifier.
#[cfg(any(feature = "server", feature = "verify"))]
pub const MAX_HAVING_LIMIT: u16 = 100;

/// Inclusive numeric bounds on one axis of an indexed tree — the resolved
/// form of a `HAVING <aggregate> <operator> <value>` clause.
///
/// One variant per axis because the three axes have three value types
/// (`u64` count, `i64` sum, `i128` fixed-point average) and the bound
/// arithmetic (operator translation, successor/predecessor at exclusive
/// bounds) must be exact in the axis's own domain. Both bounds are
/// **inclusive**; the operator translation in
/// [`mode_detection`] normalizes every supported operator to this form,
/// rejecting translations that would overflow (`> MAX`) or invert
/// (`lo > hi`) instead of serving a silently-empty range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub enum AxisRangeBounds {
    /// `COUNT(*) ∈ [lo, hi]`.
    Count {
        /// Inclusive lower bound.
        lo: u64,
        /// Inclusive upper bound.
        hi: u64,
    },
    /// `SUM(field) ∈ [lo, hi]`.
    Sum {
        /// Inclusive lower bound.
        lo: i64,
        /// Inclusive upper bound.
        hi: i64,
    },
    /// `AVG(field) ∈ [lo, hi]`, in the fixed-point domain described on
    /// [`super::drive_document_ranked_query::RANKED_AVG_SCALE`].
    Avg {
        /// Inclusive lower bound (fixed point).
        lo: i128,
        /// Inclusive upper bound (fixed point).
        hi: i128,
    },
}

#[cfg(any(feature = "server", feature = "verify"))]
impl AxisRangeBounds {
    /// The axis these bounds constrain.
    pub fn axis(&self) -> RankedAxis {
        match self {
            AxisRangeBounds::Count { .. } => RankedAxis::Count,
            AxisRangeBounds::Sum { .. } => RankedAxis::Sum,
            AxisRangeBounds::Avg { .. } => RankedAxis::Avg,
        }
    }

    /// The bounds as inclusive `i128` values in the axis's own domain —
    /// the form `AxisTraversal::Bounded` carries in the unified
    /// `PathQuery`. Count and sum widen losslessly; avg is already
    /// `i128` fixed point.
    pub fn inclusive_bounds_i128(&self) -> (i128, i128) {
        match *self {
            AxisRangeBounds::Count { lo, hi } => (lo as i128, hi as i128),
            AxisRangeBounds::Sum { lo, hi } => (lo as i128, hi as i128),
            AxisRangeBounds::Avg { lo, hi } => (lo, hi),
        }
    }

    /// The bounds as a byte range over the axis secondary's keyspace:
    /// `(inclusive_lower, exclusive_upper)`, with `None` for an upper
    /// bound at the axis's type maximum (no representable successor —
    /// the range is unbounded above).
    ///
    /// Secondary keys are `(sort_key ‖ group_key)` with order-preserving
    /// fixed-width sort keys, so the inclusive numeric range `[lo, hi]`
    /// is exactly the byte range `[encode(lo), encode(hi + 1))`: the
    /// exclusive upper at the *next* sort key admits every group-key
    /// suffix under `hi` and nothing above it. This mirrors — and must
    /// stay identical to — the bound construction inside grovedb's
    /// `indexed_*_range` read primitives, so the no-proof read and the
    /// proved read answer the same question.
    ///
    /// The `+ 1` cannot overflow: the `hi == MAX` case returns `None`
    /// first.
    pub fn secondary_key_bounds(&self) -> (Vec<u8>, Option<Vec<u8>>) {
        match *self {
            AxisRangeBounds::Count { lo, hi } => (
                encode_count_sort_key(lo).to_vec(),
                (hi != u64::MAX).then(|| encode_count_sort_key(hi + 1).to_vec()),
            ),
            AxisRangeBounds::Sum { lo, hi } => (
                encode_sum_sort_key(lo).to_vec(),
                (hi != i64::MAX).then(|| encode_sum_sort_key(hi + 1).to_vec()),
            ),
            AxisRangeBounds::Avg { lo, hi } => (
                encode_avg_sort_key(lo).to_vec(),
                (hi != i128::MAX).then(|| encode_avg_sort_key(hi + 1).to_vec()),
            ),
        }
    }

    /// The Merk query over the axis secondary that reads exactly these
    /// bounds, walking in the requested direction.
    ///
    /// This is the **prover/verifier-agreement artifact** of the having
    /// surface: grovedb's range-proof envelope is generated against this
    /// query and verified against the verifier's own reconstruction of
    /// it, so both sides must build it from the same bounds through this
    /// one function — a divergence surfaces as a failed verification,
    /// not a wrong answer.
    pub fn merk_query(&self, descending: bool) -> Query {
        let (lower, upper) = self.secondary_key_bounds();
        let mut query = Query::new_with_direction(!descending);
        match upper {
            Some(upper) => query.insert_range(lower..upper),
            None => query.insert_range_from(lower..),
        }
        query
    }
}

/// The resolved shape of a having-range request: the bounds (which carry
/// the axis), the walk direction, the limit, and the `(group property,
/// aggregate field)` pair the index picker needs.
///
/// Produced by [`mode_detection::detect_having_mode`]. Parallels
/// [`super::drive_document_ranked_query::DocumentRankedMode`].
///
/// Not `Eq`: the prefix pins carry [`Value`]s, whose float variant
/// keeps the type at `PartialEq`.
#[derive(Debug, Clone, PartialEq)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct DocumentHavingMode {
    /// Inclusive bounds on the aggregate, in the axis's own domain.
    pub bounds: AxisRangeBounds,
    /// Walk direction: `true` reads matching groups from the largest
    /// aggregate down. Defaults to `false` (ascending) when the request
    /// carries no `ORDER BY`.
    pub descending: bool,
    /// Maximum number of matching groups to return —
    /// `1 ..= MAX_HAVING_LIMIT`, required.
    pub limit: u16,
    /// The single `GROUP BY` property; must be the covering ranked
    /// index's **last** property.
    pub group_by_property: String,
    /// The field the aggregate applies to. Empty for `COUNT(*)`; the
    /// index's `summable` property for `SUM` / `AVG`.
    pub aggregate_field: String,
    /// The `where` prefix pins — one [`PrefixPin`] per clause, exactly
    /// one per leading property of the covering compound index, in
    /// request order (the resolver re-orders them into index order when
    /// it encodes the path). A pin normally carries one value (an `==`
    /// clause); at most one pin carries several (the single permitted
    /// branching `IN`, whose elements fan the bound out across one
    /// prefix branch each). Empty for the single-property form.
    pub prefix_pins: Vec<PrefixPin>,
}

/// A resolved having-range query. Shared by the prover and the verifier —
/// both build the grove path through
/// [`DriveDocumentHavingQuery::indexed_property_name_tree_path`] and the
/// secondary query through [`AxisRangeBounds::merk_query`], so the two
/// cannot drift on which subtree or which range the proof is about.
#[derive(Debug, Clone)]
#[cfg(any(feature = "server", feature = "verify"))]
pub struct DriveDocumentHavingQuery<'a> {
    /// The document type being filtered.
    pub document_type: DocumentTypeRef<'a>,
    /// The contract id (32 bytes). Separate from `document_type` so the
    /// verifier can build the query without the full contract.
    pub contract_id: [u8; 32],
    /// The document type name — a path segment.
    pub document_type_name: String,
    /// The covering ranked index. Its **last** property is the `GROUP
    /// BY` property and the final path segment; any leading properties
    /// are pinned by [`Self::prefix_branches`].
    pub index: &'a Index,
    /// The prefix **branches** — one inner `Vec<Vec<u8>>` of encoded
    /// path segments per branch, in index-property order; always at
    /// least one branch, several exactly when the request carried a
    /// multi-element `IN` pin. Part of the prover/verifier agreement
    /// exactly as on the ranked surface. Produced by
    /// [`super::drive_document_ranked_query::index_picker::encode_prefix_branches`]
    /// — crate-private so the resolver is the only public constructor
    /// and the encoder's invariants hold on every externally obtainable
    /// value.
    pub(crate) prefix_branches: Vec<Vec<Vec<u8>>>,
    /// Inclusive bounds on the aggregate. Carry the axis; the index must
    /// declare the matching `ranked_*` flag.
    pub bounds: AxisRangeBounds,
    /// `true` walks the secondary from the largest matching aggregate
    /// down. Tie ordering is by group key in the direction of the walk,
    /// exactly as on the ranked surface.
    pub descending: bool,
    /// Maximum number of matching groups to return. Fewer entries come
    /// back when fewer groups fall inside the bounds; that is not an
    /// error. **More matching groups than `limit` are silently cut at
    /// `limit`** — the walk stops, and nothing marks the cut. A caller
    /// can continue past *distinct* aggregate values by tightening the
    /// bound, but a cut inside a **tie** cannot be continued (see the
    /// module docs): groups tied at the boundary aggregate that fell
    /// past the limit stay unreachable until a composite-key cursor
    /// exists, so size the limit above the widest expected tie.
    pub limit: u16,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl DriveDocumentHavingQuery<'_> {
    /// The resolved prefix branches, in canonical order — one per `IN`
    /// element (a single branch without an `IN`). Read-only: the field is
    /// crate-private so the resolver's encoder invariants cannot be
    /// bypassed by construction or mutation.
    pub fn prefix_branches(&self) -> &[Vec<Vec<u8>>] {
        &self.prefix_branches
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl DriveDocumentHavingQuery<'_> {
    /// Path of one branch's terminal property-name tree — identical to
    /// the ranked surface's path (including the pinned-prefix segments
    /// of a compound index), because both read the same indexed
    /// tree(s). See
    /// [`DriveDocumentRankedQuery::indexed_property_name_tree_path`](super::drive_document_ranked_query::DriveDocumentRankedQuery::indexed_property_name_tree_path).
    pub fn indexed_property_name_tree_path(&self, branch: usize) -> Result<Vec<Vec<u8>>, Error> {
        let prefix_values =
            self.prefix_branches
                .get(branch)
                .ok_or(Error::Drive(DriveError::NotSupported(
                    "ranked and having-range queries addressed a prefix branch outside the \
                 query's resolved branch set",
                )))?;
        indexed_property_name_tree_path_for_index(
            &self.contract_id,
            &self.document_type_name,
            self.index,
            prefix_values,
        )
    }
}

/// Resolve a validated [`DocumentHavingMode`] against a document type's
/// indexes into the executable [`DriveDocumentHavingQuery`]: pick the
/// covering index (shared with the ranked surface — both read the same
/// indexed tree), encode the prefix pins into prefix **branches** (one
/// branch for all-`==` pins, one branch per element of the single
/// permitted `IN`), and assemble the query.
///
/// The **one** resolution path for the having surface, mirroring
/// [`super::drive_document_ranked_query::index_picker::resolve_ranked_query_for_mode`]:
/// the server's executors and the SDK's proof helpers both call it, so a
/// proof and an unproven read (and the client's verification) are about
/// the same subtree by construction.
///
/// `indexes` is threaded in separately for the same lifetime reason as
/// the ranked resolver: the returned query's `&'a Index` must outlive
/// this frame. Callers pass `document_type.indexes()`.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn resolve_having_query_for_mode<'a>(
    contract_id: [u8; 32],
    document_type: DocumentTypeRef<'a>,
    document_type_name: String,
    indexes: &'a BTreeMap<String, Index>,
    mode: &DocumentHavingMode,
    platform_version: &PlatformVersion,
) -> Result<DriveDocumentHavingQuery<'a>, Error> {
    let axis = mode.bounds.axis();
    let pin_fields: Vec<String> = mode
        .prefix_pins
        .iter()
        .map(|pin| pin.field.clone())
        .collect();
    let index = find_ranked_index_for_axis(
        indexes,
        &mode.group_by_property,
        &pin_fields,
        axis,
        &mode.aggregate_field,
    )
    .ok_or_else(|| {
        Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
            no_covering_index_message(
                "having-range",
                axis,
                &mode.group_by_property,
                &mode.prefix_pins,
                &mode.aggregate_field,
            ),
        ))
    })?;
    let prefix_branches =
        encode_prefix_branches(document_type, index, &mode.prefix_pins, platform_version)?;
    Ok(DriveDocumentHavingQuery {
        document_type,
        contract_id,
        document_type_name,
        index,
        prefix_branches,
        bounds: mode.bounds,
        descending: mode.descending,
        limit: mode.limit,
    })
}
