//! `DriveDocumentAverageQuery` — Drive's average-query surface.
//!
//! Parallels [`crate::query::drive_document_sum_query`] for the
//! averaging surface. Averages are NOT computed server-side: every
//! response carries a `(count, sum)` pair (atomic per group), and the
//! client divides to obtain the average. This preserves full
//! precision and lets callers pick their own representation
//! (integer-truncated, floating-point, decimal); pre-dividing on the
//! server would force a choice that loses information.
//!
//! The grovedb primitive backing this is `AggregateCountAndSumOnRange`
//! (added in grovedb PR 670 alongside the `ProvableCountProvableSumTree`
//! / PCPS element variant) — one root-hash-committed traversal returns
//! both metrics together. See
//! [`book/src/drive/average-index-examples.md`](../../../../../book/src/drive/average-index-examples.md)
//! for the design and the grades-contract worked example.
//!
//! Wired end-to-end: the dispatcher routes prove-true requests to the
//! PCPS / primary-key proof executors, and prove-false requests to the
//! joint single-walk count-and-sum dispatcher at
//! [`crate::query::drive_document_count_and_sum_query`]. Both paths
//! read `(count, sum)` from each visited count-sum-bearing element in
//! a single grovedb walk — see the
//! [`drive_dispatcher`](drive_dispatcher) module docstring for the
//! routing details and
//! [`crate::query::drive_document_count_and_sum_query`] for the
//! no-prove perf / atomicity contract.

#[cfg(feature = "server")]
pub mod drive_dispatcher;

#[cfg(feature = "server")]
use crate::query::ResolvedTimeRange;
use crate::query::{OrderClause, WhereClause};

#[cfg(feature = "server")]
use crate::config::DriveConfig;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(feature = "server")]
use dpp::data_contract::DataContract;

/// What kind of average-query the dispatcher should run. Parallels
/// [`crate::query::drive_document_sum_query::SumMode`].
///
/// The four variants correspond to the four response shapes:
/// - `Aggregate` → `DocumentAverageResponse::Aggregate { count, sum }`
///   (one pair across all matched docs)
/// - `GroupByIn` → `DocumentAverageResponse::Entries(Vec<AverageEntry>)`
///   (one entry per `In` value, each with its own `(count, sum)`)
/// - `GroupByRange` → `Entries` with one entry per distinct in-range value
/// - `GroupByCompound` → `Entries` with one entry per `(in_key, key)`
///   pair (compound `In + range`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AverageMode {
    /// One `(count, sum)` pair across all matched documents.
    Aggregate,
    /// One pair per `In` value (cartesian fan-out at the `In`'s position).
    GroupByIn,
    /// One pair per distinct value in a range.
    GroupByRange,
    /// One pair per `(In-value, range-value)` pair.
    GroupByCompound,
}

/// A single per-key average entry. Carries BOTH count and sum so the
/// client can divide; the server intentionally doesn't pre-divide
/// (see the module docstring).
///
/// - `in_key` carries the In value for compound `(In, range)` queries;
///   `None` for flat queries.
/// - `key` carries the terminator value (the range-key or the In
///   single value, depending on shape).
/// - `count` is the document count matching this key.
/// - `sum` is the aggregated property value matching this key.
///
/// Both `count` and `sum` are `Option<_>` so the dispatcher can emit
/// proven-absent entries when
/// `absence_proofs_for_non_existing_searched_keys` is configured,
/// mirroring the same three-valued pattern as `SumEntry` /
/// `SplitCountEntry`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AverageEntry {
    /// In-prefix value when the query is compound (`In` on a prefix
    /// property + range on the terminator). `None` for flat queries.
    pub in_key: Option<Vec<u8>>,
    /// The terminator key value (the value of the index's last covered
    /// property within the query).
    pub key: Vec<u8>,
    /// Matched-document count for this key. `Some(n)` for matched
    /// keys; `None` for keys proven absent.
    pub count: Option<u64>,
    /// Aggregated `sum_property` value for this key. `Some(n)` for
    /// matched keys; `None` for keys proven absent.
    pub sum: Option<i64>,
}

/// Server-side request input for the average dispatcher. Mirrors
/// [`crate::query::drive_document_sum_query::DocumentSumRequest`]
/// — same fields, same semantics; the response carries `(count, sum)`
/// pairs instead of a single sum.
#[cfg(feature = "server")]
#[derive(Clone, Debug)]
pub struct DocumentAverageRequest<'a> {
    /// The data contract this document type belongs to.
    pub contract: &'a DataContract,
    /// The document type whose summable indexes will be picked from.
    pub document_type: DocumentTypeRef<'a>,
    /// The integer property to average. Must match the doctype-level
    /// `documents_summable` (when set) and every covering index's
    /// `summable: "<x>"` declaration; the dispatcher rejects mismatches
    /// at parse time. Averages reuse the sum-tree index machinery —
    /// no separate `averageable` flag exists or is needed (the same
    /// `CountSumTree` / PCPS element backs both).
    pub sum_property: String,
    /// Structured where-clauses.
    pub where_clauses: Vec<WhereClause>,
    /// The fields among `where_clauses` whose equality clause was produced by
    /// `IN_TIME_RANGE` resolution rather than written by the caller. Same
    /// contract and same purpose as
    /// [`crate::query::DriveDocumentQuery::resolved_time_ranges`]:
    /// it is what gates which indexes the sum pickers (which average rides)
    /// may select.
    pub resolved_time_ranges: Vec<ResolvedTimeRange>,
    /// Structured order-clauses.
    pub order_clauses: Vec<OrderClause>,
    /// The average mode requested.
    pub mode: AverageMode,
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
    /// the (count, sum) pairs server-side.
    pub prove: bool,
    /// Pointer to the drive config, used for limit defaults.
    pub drive_config: &'a DriveConfig,
}

/// Server-side response from the average dispatcher. Parallels
/// [`crate::query::drive_document_sum_query::DocumentSumResponse`]
/// — same outer shape; the `Aggregate` and `Entries` payloads carry
/// `(count, sum)` instead of just `sum`.
#[cfg(feature = "server")]
#[derive(Clone, Debug)]
pub enum DocumentAverageResponse {
    /// A single `(count, sum)` pair across all matched documents.
    /// Client computes `avg = sum / count`.
    Aggregate {
        /// Total matched-document count.
        count: u64,
        /// Total aggregated value of `sum_property`.
        sum: i64,
    },
    /// One entry per `In`-value or per distinct in-range value.
    Entries(Vec<AverageEntry>),
    /// Serialized grovedb proof bytes the client verifies with
    /// `GroveDb::verify_aggregate_count_and_sum_query` (range path)
    /// or the appropriate point-lookup verifier.
    Proof(Vec<u8>),
}
