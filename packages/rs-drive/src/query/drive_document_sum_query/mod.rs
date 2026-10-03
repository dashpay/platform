//! `DriveDocumentSumQuery` — Drive's sum-query surface.
//!
//! Parallels [`crate::query::drive_document_count_query`] for the sum-tree
//! family added in v3 (alongside grovedb PR 670's
//! `Element::ProvableCountSumTree`). The high-level shape mirrors count's
//! exactly:
//!
//! - [`DocumentSumRequest`] carries the request (contract + document_type
//!   + sum_property + where/order/mode/limit/prove).
//! - [`DocumentSumResponse`] carries one of three response shapes
//!   (`Aggregate(i64)` / `Entries(Vec<SumEntry>)` / `Proof(Vec<u8>)`),
//!   picked by the dispatcher from query shape + flags.
//! - [`SumMode`] selects which executor handles the query
//!   (Aggregate / GroupByIn / GroupByRange / GroupByCompound).
//! - [`DriveDocumentSumQuery`] is the compiled query object passed to
//!   path-query builders + verifier wrappers; shared by prover and
//!   verifier as the single source of truth on the path-query shape
//!   (same pattern count uses).
//!
//! The sum-specific wrinkle vs count: every sum request carries a
//! `sum_property` field naming the integer property to aggregate. The
//! dispatcher validates that the chosen covering index `summable: "<x>"`
//! matches the request's `sum_property`, and that the doctype-level
//! `documents_summable: "<x>"` (if set) also matches. See
//! `book/src/drive/document-sum-trees.md` for the design rationale.
//!
//! The bench at
//! [`packages/rs-drive/benches/document_sum_worst_case.rs`](../../../../../benches/document_sum_worst_case.rs)
//! targets these public types and the dispatcher entry — Q1–Q9 from
//! the chapter all roundtrip on the real Drive.

#[cfg(feature = "server")]
pub mod drive_dispatcher;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod index_picker;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod mode_detection;

#[cfg(any(feature = "server", feature = "verify"))]
pub mod path_query;

#[cfg(feature = "server")]
pub mod execute_point_lookup;

#[cfg(feature = "server")]
pub mod execute_range_sum;

#[cfg(feature = "server")]
pub mod executors;

#[cfg(test)]
mod tests;

#[cfg(feature = "server")]
use crate::query::ResolvedTimeRange;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::query::{WhereClause, WhereOperator};

#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::{DocumentTypeRef, Index};

#[cfg(feature = "server")]
use crate::config::DriveConfig;
#[cfg(feature = "server")]
use crate::query::OrderClause;
#[cfg(feature = "server")]
use dpp::data_contract::DataContract;

/// Failsafe cap on per-`In`-value fan-out, mirroring
/// [`crate::query::drive_document_count_query::MAX_LIMIT_AS_FAILSAFE`].
/// `WhereClause::in_values()` already caps each `In` clause at 100
/// values, so this 1024 ceiling exists only as a defensive guard against
/// pathological input that slipped past the upstream validator.
pub const MAX_LIMIT_AS_FAILSAFE: u32 = 1024;

/// Platform-wide cap on the outer walk of a carrier-aggregate
/// range-outer sum proof, mirroring count's
/// `MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`. The carrier-aggregate
/// shape is the sum analog of count's G8 — single proof carrying
/// per-bucket aggregated sums for an outer range × inner range query.
/// Bounded so a single proof's outer enumeration can't be made
/// pathological by a caller.
pub const MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT: u32 = 10;

/// What kind of sum-query the dispatcher should run. Parallels
/// [`crate::query::drive_document_count_query::CountMode`].
///
/// The four variants correspond to the four response shapes:
/// - `Aggregate` → `DocumentSumResponse::Aggregate(i64)` (one sum)
/// - `GroupByIn` → `DocumentSumResponse::Entries(Vec<SumEntry>)`
///   (one entry per `In` value)
/// - `GroupByRange` → `Entries` with one entry per distinct
///   in-range value
/// - `GroupByCompound` → `Entries` with one entry per `(in_key, key)`
///   pair (compound `In + range`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SumMode {
    /// One sum across all matched documents.
    Aggregate,
    /// One sum per `In` value (cartesian fan-out at the `In`'s position).
    GroupByIn,
    /// One sum per distinct value in a range.
    GroupByRange,
    /// One sum per `(In-value, range-value)` pair.
    GroupByCompound,
}

/// The lower-level routing decision the dispatcher reaches after
/// detecting the query's shape. Parallels count's `DocumentCountMode`.
///
/// Where `SumMode` is *what the caller asked for* (an externally-facing
/// classification), `DocumentSumMode` is *which executor will run*
/// (an internally-facing classification that maps onto a specific
/// grovedb primitive). The dispatcher's job is to translate from the
/// former to the latter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentSumMode {
    /// `Aggregate` + no `where` → primary-key SumTree fast path.
    Total,
    /// `Aggregate` or `GroupByIn` + Equal/In `where` clauses fully
    /// covering a `summable: "<prop>"` index.
    PerInValue,
    /// `Aggregate` + range `where` → `AggregateSumOnRange` no-prove
    /// path (or its proven counterpart on the prove path).
    RangeNoProof,
    /// `Aggregate` + range `where` + `prove = true` → grovedb
    /// `AggregateSumOnRange` proof primitive.
    RangeProof,
    /// `GroupByRange` (or `GroupByCompound` distinct mode) → per-key
    /// `KVSum` walk with proof.
    RangeDistinctProof,
    /// Point-lookup with proof.
    PointLookupProof,
    /// Carrier-aggregate: outer In/range with inner range, sum
    /// committed per outer bucket. Sum analog of count's
    /// `RangeAggregateCarrierProof`.
    RangeAggregateCarrierProof,
}

/// A single per-key sum entry, parallels count's `SplitCountEntry`.
///
/// - `in_key` carries the In value for compound `(In, range)` queries;
///   `None` for flat queries.
/// - `key` carries the terminator value (the range-key or the In
///   single value, depending on shape).
/// - `sum` carries the aggregated property value; `Some(n)` for a
///   matched key, `None` for a key explicitly proven absent (mirrors
///   count's three-valued `count`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SumEntry {
    /// In-prefix value when the query is compound (`In` on a prefix
    /// property + range on the terminator). `None` for flat queries.
    pub in_key: Option<Vec<u8>>,
    /// The terminator key value (the value of the index's last covered
    /// property within the query).
    pub key: Vec<u8>,
    /// The aggregated `sum_property` value at that key. `Some(n)` for
    /// matched keys; `None` for keys proven absent (the dispatcher
    /// emits `None`-sum entries when
    /// `absence_proofs_for_non_existing_searched_keys` is configured).
    pub sum: Option<i64>,
}

/// Server-side request input for the sum dispatcher. Mirrors
/// [`crate::query::drive_document_count_query::DocumentCountRequest`]
/// with the addition of the `sum_property` field.
#[cfg(feature = "server")]
#[derive(Clone, Debug)]
pub struct DocumentSumRequest<'a> {
    /// The data contract this document type belongs to.
    pub contract: &'a DataContract,
    /// The document type whose summable indexes will be picked from.
    pub document_type: DocumentTypeRef<'a>,
    /// The integer property to sum. Must match the doctype-level
    /// `documents_summable` (when set) and every covering index's
    /// `summable: "<x>"` declaration; the dispatcher rejects mismatches
    /// at parse time.
    pub sum_property: String,
    /// Structured where-clauses (parsed via
    /// [`drive_dispatcher::where_clauses_from_value`] from the
    /// wire-CBOR shape).
    pub where_clauses: Vec<WhereClause>,
    /// The fields among `where_clauses` whose equality clause was produced by
    /// `IN_TIME_RANGE` resolution rather than written by the caller. Same
    /// contract and same purpose as
    /// [`crate::query::DriveDocumentQuery::resolved_time_ranges`]:
    /// it is what gates which indexes the sum pickers may select.
    pub resolved_time_ranges: Vec<ResolvedTimeRange>,
    /// Structured order-clauses (parsed via
    /// [`drive_dispatcher::order_clauses_from_value`]).
    pub order_clauses: Vec<OrderClause>,
    /// The sum mode requested.
    pub mode: SumMode,
    /// Optional cap on the number of entries returned in `Entries`-mode
    /// responses.
    ///
    /// **Fallback differs between the no-proof and prove paths**:
    ///
    /// - **No-proof path**: unset `limit` falls back to
    ///   [`crate::config::DriveConfig::default_query_limit`] (the
    ///   operator-tunable runtime value); explicit `limit >
    ///   max_query_limit` is clamped to `max_query_limit`. There's
    ///   no consensus-verification step on no-proof responses, so
    ///   operator-tunable defaults are safe here.
    /// - **Prove path**: unset `limit` falls back to
    ///   [`crate::config::DEFAULT_QUERY_LIMIT`] (the compile-time
    ///   constant the SDK verifier also reads), explicitly NOT
    ///   `drive_config.default_query_limit`. An explicit `limit >
    ///   max_query_limit` is **rejected** with
    ///   [`crate::error::query::QuerySyntaxError::InvalidLimit`]
    ///   rather than clamped, so a tuned operator default or an
    ///   over-max request can't byte-differ the
    ///   `SizedQuery::limit` the SDK reconstructs for merk-root
    ///   verification. See the
    ///   [`drive_dispatcher`]'s `RangeDistinctProof` /
    ///   `RangeAggregateCarrierProof` arms for the
    ///   validate-don't-clamp policy, mirrored from count's
    ///   prove-path arms.
    pub limit: Option<u32>,
    /// Whether to return a `Proof(Vec<u8>)` instead of materializing
    /// the aggregate/entries server-side.
    pub prove: bool,
    /// Pointer to the drive config, used for limit defaults.
    pub drive_config: &'a DriveConfig,
}

/// Server-side response from the sum dispatcher. Parallels count's
/// `DocumentCountResponse`.
#[cfg(feature = "server")]
#[derive(Clone, Debug)]
pub enum DocumentSumResponse {
    /// A single aggregated sum across all matched documents.
    Aggregate(i64),
    /// One entry per `In`-value or per distinct in-range value.
    Entries(Vec<SumEntry>),
    /// Serialized grovedb proof bytes the client verifies with
    /// `GroveDb::verify_query` (point-lookup proofs) or
    /// `GroveDb::verify_aggregate_sum_query` (range-aggregate proofs).
    Proof(Vec<u8>),
}

/// Compiled sum-query object. Shared by prover and verifier — both
/// build the same `PathQuery` via the path-query helpers on this
/// struct, so the prover and the verifier can't drift on shape.
/// Parallels count's `DriveDocumentCountQuery`.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Clone, Debug)]
pub struct DriveDocumentSumQuery<'a> {
    /// The document type whose sum tree we're querying.
    pub document_type: DocumentTypeRef<'a>,
    /// The data contract id (separated from `document_type` so the
    /// verifier-side construction doesn't need the full contract).
    pub contract_id: [u8; 32],
    /// The document type name (used to construct the index path).
    pub document_type_name: String,
    /// The covering index. Either the index whose `summable` flag
    /// matches the request's `sum_property` (point lookup / aggregate
    /// case), or the index whose `range_summable` matches (range
    /// case). The doctype-primary-key fast path stores this as a
    /// sentinel — see `path_query.rs`'s `primary_key_sum_path_query`.
    pub index: &'a Index,
    /// The structured where clauses.
    pub where_clauses: Vec<WhereClause>,
    /// The sum target property. Validated against the index's
    /// `summable` and the doctype's `documents_summable` at dispatch
    /// time.
    pub sum_property: String,
}

/// Storage-walk shape for a server-side range sum.
#[cfg(feature = "server")]
#[derive(Clone, Copy, Debug, Default)]
pub enum RangeSumWalkMode {
    /// Return one aggregate sum for the range.
    #[default]
    Aggregate,
    /// Return distinct sums, bounded by the supplied storage-walk limit.
    Distinct(u16),
}

/// Server-side range-sum executor options, parallels
/// [`crate::query::drive_document_count_query::RangeCountOptions`].
#[cfg(feature = "server")]
#[derive(Clone, Debug, Default)]
pub struct RangeSumOptions {
    /// Select aggregate execution or a compile-time bounded distinct walk.
    pub walk_mode: RangeSumWalkMode,
    /// `Some(n)` caps the carrier walk for compound `(In, range)`
    /// shapes at n entries. `None` accepts the platform-wide
    /// `MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`.
    pub carrier_outer_limit: Option<u32>,
    /// Whether the carrier walk iterates ascending (`true`) or
    /// descending (`false`); flows into grovedb's `Query.left_to_right`.
    pub left_to_right: bool,
}

/// Helper used by the verifier-side path-query rebuild to match the
/// shape the prover used. Same role as count's analog helper — we
/// don't want the prover and verifier to drift on which operator
/// classification triggers which executor.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn is_range_operator(op: WhereOperator) -> bool {
    matches!(
        op,
        WhereOperator::GreaterThan
            | WhereOperator::GreaterThanOrEquals
            | WhereOperator::LessThan
            | WhereOperator::LessThanOrEquals
            | WhereOperator::Between
            | WhereOperator::BetweenExcludeBounds
            | WhereOperator::BetweenExcludeLeft
            | WhereOperator::BetweenExcludeRight
            | WhereOperator::StartsWith
    )
}

/// True if the `WhereOperator` is supported on a summable index for
/// the executor pickers. Parallels count's `is_indexable_for_count`.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn is_indexable_for_sum(op: WhereOperator) -> bool {
    is_range_operator(op) || matches!(op, WhereOperator::Equal | WhereOperator::In)
}
