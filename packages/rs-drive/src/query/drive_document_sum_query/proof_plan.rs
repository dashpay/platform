//! The request-only half of a proved sum or average: which proof answers
//! the request, what that proof reads, and the limit and walk direction its
//! path query carries.
//!
//! A sum and an average read the same trees through the same path queries;
//! an average's range and carrier proofs read each element's count beside
//! its sum. They share [`SumProofPlan`], and each surface keeps its own
//! planner, since the server routes a sum by its resolved
//! [`DocumentSumMode`] and an average by its mode and whether a range
//! constrains it, each with its own refusals: this module plans a sum, and
//! [`crate::query::drive_document_average_query::proof_plan`] an average.
//! The server's dispatchers
//! prove what the plan names, and the SDK's verifiers rebuild the path query
//! from the same plan, so the two cannot read one request differently; a
//! client refuses before sending what the plan refuses, in the server's
//! words. The planners read no version table, so every protocol version
//! that serves a proved sum or average plans the same proof for the same
//! request.

use super::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use super::{DocumentSumMode, DriveDocumentSumQuery};
use crate::config::DEFAULT_QUERY_LIMIT;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{OrderClause, ResolvedTimeRange, WhereClause};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::version::PlatformVersion;
use grovedb::PathQuery;

/// What a proved sum or average reads, decided from its request alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SumProofPlan<'a> {
    /// An unfiltered total over a type summed by the requested property (for
    /// an average, also `documentsCountable`): the type's primary-key sum
    /// tree.
    PrimaryKeySumTree,
    /// `Equal` / `In` clauses on a summable index: one sum-bearing element
    /// per matched branch.
    PointLookup(&'a Index),
    /// A range on a range-summable index, ungrouped: one aggregate over the
    /// range.
    Range(&'a Index),
    /// A range grouped by its values: one entry per distinct value, walking
    /// at most `limit` of them.
    RangeDistinct {
        /// The range-summable index the walk reads.
        index: &'a Index,
        /// The `SizedQuery` limit of the walk.
        limit: u16,
        /// The walk's direction, `Query::left_to_right`.
        left_to_right: bool,
    },
    /// A range grouped by an outer `In` or range: one aggregate per outer
    /// key.
    RangeAggregateCarrier {
        /// The range-summable index the carrier reads.
        index: &'a Index,
        /// The `SizedQuery` limit of the outer walk; `None` leaves it
        /// unbounded.
        limit: Option<u16>,
        /// The outer walk's direction, `Query::left_to_right`.
        left_to_right: bool,
    },
}

impl DriveDocumentSumQuery<'_> {
    /// The [`SumProofPlan`] of a proved sum of `sum_property` over
    /// `document_type`.
    ///
    /// `where_clauses` are the request's clauses as the dispatcher reads them
    /// (typed array elements keyed, canonicalized), `resolved_time_ranges`
    /// their window provenance, and `mode` the proof mode
    /// [`detect_sum_mode_from_inputs`](super::mode_detection::detect_sum_mode_from_inputs)
    /// resolved them to. `order_clauses` and `limit` are the request's,
    /// `limit` `None` when unset; a distinct or carrier walk runs in the
    /// first order clause's direction, ascending by default.
    ///
    /// `max_query_limit` is the node's configured cap on a proved walk's
    /// limit, checked before the index is picked. A client, which cannot
    /// know a node's configuration, passes `None`; the node refuses a limit
    /// above its own cap.
    #[allow(clippy::too_many_arguments)]
    pub fn proof_plan<'a>(
        document_type: &'a DocumentTypeRef<'_>,
        where_clauses: &[WhereClause],
        resolved_time_ranges: &[ResolvedTimeRange],
        sum_property: &str,
        mode: DocumentSumMode,
        order_clauses: &[OrderClause],
        limit: Option<u32>,
        max_query_limit: Option<u16>,
    ) -> Result<SumProofPlan<'a>, Error> {
        let left_to_right = order_clauses.first().map(|c| c.ascending).unwrap_or(true);
        let range_index = || {
            find_range_summable_index_for_where_clauses(
                document_type.indexes(),
                where_clauses,
                sum_property,
                resolved_time_ranges,
            )
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "range sum requires a `rangeSummable: true` index whose last \
                     property matches the range field"
                        .to_string(),
                ))
            })
        };
        match mode {
            DocumentSumMode::RangeProof => Ok(SumProofPlan::Range(range_index()?)),
            DocumentSumMode::RangeDistinctProof => {
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
                             distinct-walk path (GROUP BY a range field, SUM); \
                             reduce the requested limit or use prove = false",
                            effective_limit, max_query_limit
                        ))));
                    }
                }
                let limit = u16::try_from(effective_limit).map_err(|_| {
                    Error::Query(QuerySyntaxError::Unsupported(format!(
                        "limit {} exceeds u16::MAX for range-distinct sum proof",
                        effective_limit
                    )))
                })?;
                Ok(SumProofPlan::RangeDistinct {
                    index: range_index()?,
                    limit,
                    left_to_right,
                })
            }
            DocumentSumMode::PointLookupProof => {
                // An unfiltered sum over a type summed by the requested
                // property proves the primary-key sum tree directly.
                if where_clauses.is_empty()
                    && document_type
                        .documents_summable()
                        .map(|p| p == sum_property)
                        .unwrap_or(false)
                {
                    return Ok(SumProofPlan::PrimaryKeySumTree);
                }
                find_summable_index_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    sum_property,
                    resolved_time_ranges,
                )
                .map(SumProofPlan::PointLookup)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "prove sum requires a `summable: \"<prop>\"` index whose properties \
                         exactly match the where clause fields and whose summed property \
                         matches the request's `sum_property`, or `documentsSummable: \
                         \"<prop>\"` on the document type for unfiltered total sums — same \
                         requirement as the no-proof path"
                            .to_string(),
                    ))
                })
            }
            DocumentSumMode::RangeAggregateCarrierProof => {
                // Validate-don't-clamp, as for the distinct walk; an unset
                // limit stays unset (an unbounded outer walk) rather than
                // falling back to a default.
                let limit = limit
                    .map(|l| {
                        if let Some(max_query_limit) = max_query_limit {
                            if l > max_query_limit as u32 {
                                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                                    "limit {} exceeds max_query_limit {} on the prove + \
                                         carrier-aggregate path (GROUP BY In + range, SUM); \
                                         reduce the requested limit or use prove = false",
                                    l, max_query_limit
                                ))));
                            }
                        }
                        u16::try_from(l).map_err(|_| {
                            Error::Query(QuerySyntaxError::Unsupported(format!(
                                "limit {} exceeds u16::MAX for carrier-aggregate sum proof",
                                l
                            )))
                        })
                    })
                    .transpose()?;
                let index = find_range_summable_index_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    sum_property,
                    resolved_time_ranges,
                )
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "carrier-aggregate sum requires a `rangeSummable: true` index whose first \
                         property carries the outer In or range clause and whose last property \
                         carries the inner ASOR range clause"
                            .to_string(),
                    ))
                })?;
                Ok(SumProofPlan::RangeAggregateCarrier {
                    index,
                    limit,
                    left_to_right,
                })
            }
            DocumentSumMode::Total
            | DocumentSumMode::PerInValue
            | DocumentSumMode::RangeNoProof => {
                Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a sum proof plan is made only for a proof mode; mode detection with \
                     prove = true never returns a no-proof mode",
                )))
            }
        }
    }
}

impl SumProofPlan<'_> {
    /// The path query a proof of this plan answers. The server proves it
    /// (`Drive::execute_document_sum_proof`), and a client builds it from the
    /// same plan before sending and before it verifies an answer, so a
    /// request whose path query cannot be built is refused in the server's
    /// words.
    ///
    /// `with_counts` is whether the request is an average, whose range and
    /// carrier proofs read each element's count beside its sum; its point
    /// lookups and distinct walks read the count-sum-bearing elements the
    /// index's terminators already hold. The other arguments are the
    /// request's, as the planner read them.
    #[allow(clippy::too_many_arguments)]
    pub fn path_query(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        sum_property: String,
        with_counts: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let sum_query = |index| DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            sum_property,
        };
        match *self {
            SumProofPlan::PrimaryKeySumTree => {
                Ok(DriveDocumentSumQuery::primary_key_sum_path_query(
                    contract_id,
                    document_type.name(),
                ))
            }
            SumProofPlan::PointLookup(index) => {
                sum_query(index).point_lookup_sum_path_query(platform_version)
            }
            SumProofPlan::Range(index) if with_counts => {
                sum_query(index).aggregate_count_and_sum_path_query(platform_version)
            }
            SumProofPlan::Range(index) => {
                sum_query(index).aggregate_sum_path_query(platform_version)
            }
            SumProofPlan::RangeDistinct {
                index,
                limit,
                left_to_right,
            } => sum_query(index).distinct_sum_path_query(
                Some(limit),
                left_to_right,
                platform_version,
            ),
            SumProofPlan::RangeAggregateCarrier {
                index,
                limit,
                left_to_right,
            } if with_counts => sum_query(index).carrier_aggregate_count_and_sum_path_query(
                limit,
                left_to_right,
                platform_version,
            ),
            SumProofPlan::RangeAggregateCarrier {
                index,
                limit,
                left_to_right,
            } => sum_query(index).carrier_aggregate_sum_path_query(
                limit,
                left_to_right,
                platform_version,
            ),
        }
    }
}
