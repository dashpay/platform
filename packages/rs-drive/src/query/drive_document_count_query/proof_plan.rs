//! The request-only half of a proved count: which proof answers the
//! request, what that proof reads, and the limit and walk direction its
//! path query carries.
//!
//! The server's dispatcher proves what the plan names, and the SDK's
//! verifier rebuilds the path query from the same plan, so the two cannot
//! read one request differently; a client refuses before sending what the
//! plan refuses, in the server's words.

use super::{DocumentCountMode, DriveDocumentCountQuery, QuerySyntaxError};
use crate::config::DEFAULT_QUERY_LIMIT;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{OrderClause, ResolvedTimeRange, WhereClause};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

/// What a proved count reads, decided from its request alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CountProofPlan<'a> {
    /// An unfiltered count of a `documentsCountable` document type: the
    /// type's primary-key count tree.
    PrimaryKeyCountTree,
    /// `Equal` / `In` clauses on a countable index: one count tree per
    /// matched branch.
    PointLookup(&'a Index),
    /// A range on a range-countable index, ungrouped: one aggregate over the
    /// range.
    Range(&'a Index),
    /// A range grouped by its values: one count per distinct value, walking
    /// at most `limit` of them.
    RangeDistinct {
        /// The range-countable index the walk reads.
        index: &'a Index,
        /// The `SizedQuery` limit of the walk.
        limit: u16,
        /// The walk's direction, `Query::left_to_right`.
        left_to_right: bool,
    },
    /// A range grouped by an outer `In` or range: one aggregate per outer
    /// key.
    RangeAggregateCarrier {
        /// The range-countable index the carrier reads.
        index: &'a Index,
        /// The `SizedQuery` limit of the outer walk; `None` for an `In`
        /// outer, whose values bound it.
        limit: Option<u16>,
        /// The outer walk's direction, `Query::left_to_right`.
        left_to_right: bool,
    },
}

impl DriveDocumentCountQuery<'_> {
    /// The [`CountProofPlan`] of a proved count over `document_type`.
    ///
    /// `where_clauses` are the request's clauses as the dispatcher reads them
    /// (typed array elements keyed, canonicalized), `resolved_time_ranges`
    /// their window provenance, and `mode` the proof mode
    /// [`Self::detect_mode_versioned`] resolved them to. `order_clauses` and
    /// `limit` are the request's, `limit` `None` when unset; a distinct or
    /// carrier walk runs in the first order clause's direction, ascending
    /// by default.
    ///
    /// `max_query_limit` is the node's configured cap on a distinct walk's
    /// limit, checked before the index is picked. A client, which cannot
    /// know a node's configuration, passes `None`; the node refuses a limit
    /// above its own cap.
    ///
    /// The plan reads no version table, so every protocol version that serves
    /// a proved count plans the same proof for the same request.
    pub fn proof_plan<'a>(
        document_type: &'a DocumentTypeRef<'_>,
        where_clauses: &[WhereClause],
        resolved_time_ranges: &[ResolvedTimeRange],
        mode: DocumentCountMode,
        order_clauses: &[OrderClause],
        limit: Option<u32>,
        max_query_limit: Option<u16>,
    ) -> Result<CountProofPlan<'a>, Error> {
        let left_to_right = order_clauses.first().map(|c| c.ascending).unwrap_or(true);
        let range_index = || {
            Self::find_range_countable_index_for_where_clauses(
                document_type.indexes(),
                where_clauses,
                resolved_time_ranges,
            )
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "range count requires a `range_countable: true` (or summableOffCountIndex) index whose last \
                     property matches the range field"
                        .to_string(),
                ))
            })
        };
        match mode {
            DocumentCountMode::RangeProof => Ok(CountProofPlan::Range(range_index()?)),
            DocumentCountMode::RangeDistinctProof => {
                // Validate-don't-clamp: the verifier rebuilds the walk's
                // `SizedQuery::limit` byte for byte, so an over-cap limit is
                // refused rather than clamped. An unset limit falls back to
                // the compile-time `DEFAULT_QUERY_LIMIT`, never the
                // operator-tunable `default_query_limit`, which a verifier
                // cannot know.
                let effective_limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT as u32);
                if let Some(max_query_limit) = max_query_limit {
                    if effective_limit > max_query_limit as u32 {
                        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                            "limit {} exceeds max_query_limit {} on the prove + \
                             distinct-walk path (GROUP BY a range field); reduce the \
                             requested limit or use prove = false",
                            effective_limit, max_query_limit
                        ))));
                    }
                }
                // Within a node's `u16` cap this always converts.
                let limit = u16::try_from(effective_limit).map_err(|_| {
                    Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "limit {effective_limit} exceeds u16::MAX; the prove-distinct path \
                         query cannot represent it"
                    )))
                })?;
                Ok(CountProofPlan::RangeDistinct {
                    index: range_index()?,
                    limit,
                    left_to_right,
                })
            }
            DocumentCountMode::PointLookupProof => {
                // An unfiltered count of a `documents_countable` type proves the
                // primary-key count tree directly, no index involved.
                if where_clauses.is_empty() && document_type.documents_countable() {
                    return Ok(CountProofPlan::PrimaryKeyCountTree);
                }
                Self::find_countable_index_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    resolved_time_ranges,
                )
                .map(CountProofPlan::PointLookup)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "prove count requires a `countable: true` (or summableOffCountIndex) index whose properties \
                         exactly match the where clause fields, a `rangeCountable: true` \
                         index they cover up to its last property, or `documentsCountable: \
                         true` on the document type for unfiltered total counts — same \
                         requirement as the no-proof path"
                            .to_string(),
                    ))
                })
            }
            DocumentCountMode::RangeAggregateCarrierProof => {
                let limit = Self::carrier_aggregate_count_limit(where_clauses, limit)?;
                let index = Self::find_range_countable_index_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    resolved_time_ranges,
                )
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "carrier-aggregate count requires a `range_countable: true` (or \
                         summableOffCountIndex) index whose first \
                         property carries the outer In or range clause and whose last property \
                         carries the inner ACOR range clause"
                            .to_string(),
                    ))
                })?;
                Ok(CountProofPlan::RangeAggregateCarrier {
                    index,
                    limit,
                    left_to_right,
                })
            }
            DocumentCountMode::Total
            | DocumentCountMode::PerInValue
            | DocumentCountMode::RangeNoProof => {
                Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a count proof plan is made only for a proof mode; mode detection with \
                     prove = true never returns a no-proof mode",
                )))
            }
        }
    }
}

impl CountProofPlan<'_> {
    /// The path query a proof of this plan answers. The server proves it
    /// (`Drive::execute_document_count_proof`), and a
    /// client builds it from the same plan before sending and before it
    /// verifies an answer, so a request whose path query cannot be built is
    /// refused in the server's words. A `summableOffCountIndex` index counts
    /// its documents through its range sums, so its range proofs read the sum
    /// surface's path queries.
    ///
    /// The other arguments are the request's, as
    /// [`DriveDocumentCountQuery::proof_plan`] read them.
    pub fn path_query(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let count_query = |index| DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
        };
        match *self {
            CountProofPlan::PrimaryKeyCountTree => {
                Ok(DriveDocumentCountQuery::primary_key_count_tree_path_query(
                    contract_id,
                    document_type.name(),
                ))
            }
            CountProofPlan::PointLookup(index) => {
                count_query(index).point_lookup_count_path_query(platform_version)
            }
            CountProofPlan::Range(index) => {
                let query = count_query(index);
                match query.counter_sums_query() {
                    Some(sums) => sums.aggregate_sum_path_query(platform_version),
                    None => query.aggregate_count_path_query(platform_version),
                }
            }
            CountProofPlan::RangeDistinct {
                index,
                limit,
                left_to_right,
            } => {
                let query = count_query(index);
                match query.counter_sums_query() {
                    Some(sums) => {
                        sums.distinct_sum_path_query(Some(limit), left_to_right, platform_version)
                    }
                    None => query.distinct_count_path_query(
                        Some(limit),
                        left_to_right,
                        platform_version,
                    ),
                }
            }
            CountProofPlan::RangeAggregateCarrier {
                index,
                limit,
                left_to_right,
            } => {
                let query = count_query(index);
                match query.counter_sums_query() {
                    Some(sums) => sums.carrier_aggregate_sum_path_query(
                        limit,
                        left_to_right,
                        platform_version,
                    ),
                    None => query.carrier_aggregate_count_path_query(
                        limit,
                        left_to_right,
                        platform_version,
                    ),
                }
            }
        }
    }
}
