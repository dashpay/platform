//! The request-only half of a proved average: which proof answers the
//! request, what that proof reads, and the limit and walk direction its
//! path query carries.
//!
//! An average reads the trees a sum reads, through the same path queries,
//! so it is planned as a [`SumProofPlan`]; its range and carrier proofs read
//! each element's count beside its sum. It is planned apart from a sum
//! because the server routes it by its [`AverageMode`] and whether a range
//! clause constrains it, not by sum's resolved mode, with its own refusals.
//! The server's average dispatcher proves what the plan names, and the SDK's
//! verifier rebuilds the path query from the same plan; a client refuses
//! before sending what the plan refuses, in the server's words. The planner
//! reads no version table, so every protocol version that serves a proved
//! average plans the same proof for the same request.

use super::AverageMode;
use crate::config::DEFAULT_QUERY_LIMIT;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_sum_query::index_picker::{
    find_range_summable_index_with_counts_for_where_clauses,
    find_summable_index_with_counts_for_where_clauses,
};
use crate::query::drive_document_sum_query::{
    is_range_operator, DriveDocumentSumQuery, SumProofPlan,
};
use crate::query::{OrderClause, ResolvedTimeRange, WhereClause};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;

impl DriveDocumentSumQuery<'_> {
    /// The [`SumProofPlan`] of a proved average of `sum_property` over
    /// `document_type`: each proof reads an element's count beside its sum.
    ///
    /// A range picks an index that is both range-countable and
    /// range-summable (one `rangeAverageable: true` declares both), no range
    /// an index that sums the property and counts its terminator. The
    /// arguments are [`DriveDocumentSumQuery::proof_plan`]'s, except that
    /// `mode` is the request's SQL-shape [`AverageMode`] and that an average
    /// picks its index *before* it checks the limit against
    /// `max_query_limit`: a request with both problems is refused for the
    /// index.
    #[allow(clippy::too_many_arguments)]
    pub fn average_proof_plan<'a>(
        document_type: &'a DocumentTypeRef<'_>,
        where_clauses: &[WhereClause],
        resolved_time_ranges: &[ResolvedTimeRange],
        sum_property: &str,
        mode: AverageMode,
        order_clauses: &[OrderClause],
        limit: Option<u32>,
        max_query_limit: Option<u16>,
    ) -> Result<SumProofPlan<'a>, Error> {
        let has_range = where_clauses
            .iter()
            .any(|wc| is_range_operator(wc.operator));
        let left_to_right = order_clauses.first().map(|c| c.ascending).unwrap_or(true);

        // An unfiltered average over a type that counts its documents and
        // sums them by the requested property proves the primary-key
        // count-sum-bearing element directly.
        if matches!(mode, AverageMode::Aggregate)
            && where_clauses.is_empty()
            && document_type.documents_countable()
            && document_type
                .documents_summable()
                .map(|p| p == sum_property)
                .unwrap_or(false)
        {
            return Ok(SumProofPlan::PrimaryKeySumTree);
        }

        match (mode, has_range) {
            (AverageMode::Aggregate | AverageMode::GroupByIn, true) => {
                let index = find_range_summable_index_with_counts_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    sum_property,
                    resolved_time_ranges,
                )
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "prove AVG requires an index that declares BOTH `rangeCountable: \
                         true` AND `rangeSummable: true` (a `rangeAverageable: true` \
                         index is the shorthand) whose last property matches the range \
                         field and whose summable property matches the request's \
                         `sum_property`"
                            .to_string(),
                    ))
                })?;
                if matches!(mode, AverageMode::Aggregate) {
                    return Ok(SumProofPlan::Range(index));
                }
                // Carrier: one (count, sum) per `In` branch. Validate-don't-
                // clamp, as for sum's carrier; an unset limit stays unset.
                let limit = limit
                    .map(|l| {
                        if let Some(max_query_limit) = max_query_limit {
                            if l > max_query_limit as u32 {
                                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                                    "limit {} exceeds max_query_limit {} on the prove + \
                                         carrier-aggregate path (GROUP BY In + range, AVG); \
                                         reduce the requested limit or use prove = false",
                                    l, max_query_limit
                                ))));
                            }
                        }
                        u16::try_from(l).map_err(|_| {
                            Error::Query(QuerySyntaxError::Unsupported(format!(
                                "limit {} exceeds u16::MAX for carrier-aggregate \
                                 count+sum (AVG) proof",
                                l
                            )))
                        })
                    })
                    .transpose()?;
                Ok(SumProofPlan::RangeAggregateCarrier {
                    index,
                    limit,
                    left_to_right,
                })
            }
            (AverageMode::GroupByRange | AverageMode::GroupByCompound, true) => {
                let index = find_range_summable_index_with_counts_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    sum_property,
                    resolved_time_ranges,
                )
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "prove distinct AVG requires an index that declares BOTH \
                         `rangeCountable: true` AND `rangeSummable: true` (a \
                         `rangeAverageable: true` index is the shorthand) whose last \
                         property matches the range field and whose summable property \
                         matches the request's `sum_property`"
                            .to_string(),
                    ))
                })?;
                // Validate-don't-clamp, as for sum's distinct walk; an unset
                // limit falls back to the compile-time `DEFAULT_QUERY_LIMIT`.
                let effective_limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT as u32);
                if let Some(max_query_limit) = max_query_limit {
                    if effective_limit > max_query_limit as u32 {
                        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                            "limit {} exceeds max_query_limit {} on the prove + distinct-walk \
                             path (GROUP BY a range field, AVG); reduce the requested limit \
                             or use prove = false",
                            effective_limit, max_query_limit
                        ))));
                    }
                }
                let limit = u16::try_from(effective_limit).map_err(|_| {
                    Error::Query(QuerySyntaxError::Unsupported(format!(
                        "limit {} exceeds u16::MAX for distinct AVG proof",
                        effective_limit
                    )))
                })?;
                Ok(SumProofPlan::RangeDistinct {
                    index,
                    limit,
                    left_to_right,
                })
            }
            // Equal / `In`, no range: one count-sum-bearing element per matched
            // branch, collapsed into one aggregate by `DocumentAverage` or kept
            // per branch by `DocumentSplitAverages` (the same proof either way).
            (AverageMode::Aggregate | AverageMode::GroupByIn, false) => {
                find_summable_index_with_counts_for_where_clauses(
                    document_type.indexes(),
                    where_clauses,
                    sum_property,
                    resolved_time_ranges,
                )
                .map(SumProofPlan::PointLookup)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                        "prove point-lookup AVG requires an index that declares BOTH \
                         `summable: \"<prop>\"` AND a countable terminator (`countable: \
                         \"countable\"` or `\"countableAllowingOffset\"`) whose properties \
                         exactly match the where clause fields"
                            .to_string(),
                    ))
                })
            }
            // A grouped walk with no range to walk: routing never sends one, so
            // only a direct Drive caller reaches this.
            (AverageMode::GroupByRange | AverageMode::GroupByCompound, false) => {
                Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "execute_document_average_request prove=true: the (mode = {:?}, has_range \
                     = {}) combination is not yet supported on the prove path. \
                     This is likely a new AverageMode variant that hasn't been wired \
                     into the prove dispatcher.",
                    mode, has_range,
                ))))
            }
        }
    }
}
