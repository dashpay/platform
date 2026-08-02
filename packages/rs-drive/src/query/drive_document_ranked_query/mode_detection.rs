//! Request-shape validation for the ranked query, and the versioned
//! `(select, group_by, having)` → [`DocumentRankedMode`] resolution.
//!
//! Pure functions on the request shape — no Drive, no contract, no
//! indexes. Available under `server` (the dispatcher validates before
//! executing) and `verify` (the SDK validates the same way before
//! attempting proof verification), so both sides agree on which requests
//! are well-formed and on the `(axis, descending, k)` triple a
//! well-formed one resolves to. Index-dependent validation ("does an
//! index actually cover this axis?") needs the document type's index map
//! and lives in [`super::index_picker`].
//!
//! Versioned through
//! `platform_version.drive.methods.document.query.detect_ranked_mode`,
//! the same way
//! [`DriveDocumentCountQuery::detect_mode_versioned`](super::super::drive_document_count_query::DriveDocumentCountQuery::detect_mode_versioned)
//! routes count's table: the accepted request grammar is a consensus-
//! adjacent contract on the query surface, so relaxing it later has to
//! land behind a method-version bump rather than changing what an
//! already-deployed protocol version accepts.

use super::{DocumentRankedMode, RankedAxis, RankedPaginationInputs, MAX_RANKED_LIMIT};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::{
    HavingAggregateFunction, HavingClause, HavingOperator, HavingRankingKind, HavingRightOperand,
};
use crate::query::projection::{SelectFunction, SelectProjection};
use crate::query::WhereClause;
use dpp::version::PlatformVersion;

/// Versioned entry point. Routes through
/// `platform_version.drive.methods.document.query.detect_ranked_mode`;
/// today only `0` is defined and maps to [`detect_ranked_mode_v0`]
/// verbatim.
pub fn detect_ranked_mode(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    where_clauses: &[WhereClause],
    pagination: RankedPaginationInputs,
    platform_version: &PlatformVersion,
) -> Result<DocumentRankedMode, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .detect_ranked_mode
    {
        0 => detect_ranked_mode_v0(select, group_by, having, where_clauses, pagination),
        version => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "detect_ranked_mode: unknown method version {version}; only 0 is supported"
        )))),
    }
}

/// v0 of the ranked request grammar.
///
/// Accepts exactly:
///
/// ```text
/// SELECT COUNT(*)   GROUP BY p HAVING COUNT(*)   IN TOP(n) | IN BOTTOM(n) | EQ MAX | EQ MIN
/// SELECT SUM(f)     GROUP BY p HAVING SUM(f)     IN TOP(n) | IN BOTTOM(n) | EQ MAX | EQ MIN
/// SELECT AVG(f)     GROUP BY p HAVING AVG(f)     IN TOP(n) | IN BOTTOM(n) | EQ MAX | EQ MIN
/// ```
///
/// with no `WHERE`, no `LIMIT` / `OFFSET` / `START AT`, `1 ≤ n ≤`
/// [`MAX_RANKED_LIMIT`], and the `HAVING` aggregate byte-equal to the
/// `SELECT` projection. `EQ TOP(1)` / `EQ BOTTOM(1)` are accepted as
/// synonyms of `EQ MAX` / `EQ MIN` because
/// [`HavingRightOperand::Ranking`]'s own contract documents them as
/// equivalent.
///
/// Everything the grammar rejects is rejected *loudly* rather than
/// normalized away: a caller who wrote a filter, a limit, or a mismatched
/// aggregate asked for something this executor cannot deliver, and
/// silently answering a different question is worse than an error.
pub fn detect_ranked_mode_v0(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    where_clauses: &[WhereClause],
    pagination: RankedPaginationInputs,
) -> Result<DocumentRankedMode, Error> {
    // ---- GROUP BY: exactly one property ----------------------------
    //
    // Ranked indexes are single-property (rs-dpp rejects compound ones
    // at parse time), and the sole property is what the secondary's
    // group keys are. Zero group_by would ask to rank a single global
    // aggregate against itself; two or more would need a compound index.
    if group_by.len() != 1 {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "ranked queries require exactly one `group_by` property (the ranked index's \
             only property); got {}. Ranked indexes are single-property — compound ranked \
             indexes are rejected at contract-parse time — so there is no compound \
             grouping to rank over.",
            group_by.len()
        ))));
    }
    let group_by_property = group_by[0].clone();
    if group_by_property.is_empty() {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(
            "ranked queries require a non-empty `group_by` property name".to_string(),
        )));
    }

    // ---- SELECT: the axis, and the field it aggregates --------------
    let (axis, aggregate_field) = match (select.function, select.field.as_str()) {
        (SelectFunction::Count, "") => (RankedAxis::Count, String::new()),
        (SelectFunction::Count, field) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "ranked queries support `COUNT(*)` but not `COUNT({field})`; a ranked \
                 count axis counts documents per group, which is what `COUNT(*)` means. \
                 Drop the field to rank by group size."
            ))));
        }
        (SelectFunction::Sum, "") | (SelectFunction::Avg, "") => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                "`SUM` / `AVG` ranked queries require a non-empty select field naming the \
                 index's `summable` property"
                    .to_string(),
            )));
        }
        (SelectFunction::Sum, field) => (RankedAxis::Sum, field.to_string()),
        (SelectFunction::Avg, field) => (RankedAxis::Avg, field.to_string()),
        (other, _) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "ranked queries support `COUNT(*)`, `SUM(field)` and `AVG(field)` \
                 selects; got {other:?}. Ranking is driven by an indexed tree's \
                 per-axis secondary, and grovedb maintains exactly those three axes."
            ))));
        }
    };

    // ---- HAVING: exactly one clause, matching the select ------------
    let clause = match having {
        [only] => only,
        _ => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "ranked queries require exactly one `having` clause carrying the ranking \
                 (`IN TOP(n)`, `IN BOTTOM(n)`, `EQ MAX` or `EQ MIN`); got {}. Multiple \
                 having clauses are implicitly ANDed, and there is no ranked primitive \
                 for an intersection of two rankings.",
                having.len()
            ))));
        }
    };

    let having_function_matches = matches!(
        (clause.aggregate.function, axis),
        (HavingAggregateFunction::Count, RankedAxis::Count)
            | (HavingAggregateFunction::Sum, RankedAxis::Sum)
            | (HavingAggregateFunction::Avg, RankedAxis::Avg)
    );
    if !having_function_matches || clause.aggregate.field != aggregate_field {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "the `having` aggregate must be the same aggregate as the `select`: select is \
             {:?}({:?}), having is {:?}({:?}). A ranked query ranks the selected \
             aggregate; ranking one aggregate while projecting another would need a \
             second axis the storage does not maintain.",
            select.function, select.field, clause.aggregate.function, clause.aggregate.field
        ))));
    }

    // ---- HAVING right operand: the ranking --------------------------
    let ranking = match &clause.right {
        HavingRightOperand::Ranking(ranking) => *ranking,
        HavingRightOperand::Value(_) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "`HAVING <aggregate> <op> <value>` (a threshold comparison) is not \
                 implemented; ranked queries take a cross-group ranking right-operand \
                 (`TOP(n)` / `BOTTOM(n)` / `MAX` / `MIN`). Thresholds need a range walk \
                 over the axis secondary, which is a separate primitive."
                    .to_string(),
            )));
        }
    };

    // Direction comes from the ranking kind; `k` comes from `n` for the
    // set-valued kinds and is pinned to 1 for the scalar ones.
    let descending = match ranking.kind {
        HavingRankingKind::Top | HavingRankingKind::Max => true,
        HavingRankingKind::Bottom | HavingRankingKind::Min => false,
    };

    let k = match ranking.kind {
        HavingRankingKind::Max | HavingRankingKind::Min => {
            // `n` is wire-permitted on Min/Max for forward compatibility
            // but is malformed today — see `HavingRanking::n`.
            if ranking.n.is_some() {
                return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                    "`{:?}` ranking must not carry `n` (it selects exactly one group); \
                     use `TOP(n)` / `BOTTOM(n)` for a set of size n",
                    ranking.kind
                ))));
            }
            if !matches!(clause.operator, HavingOperator::Equal) {
                return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "`{:?}` ranking is supported with the `=` having operator only; got \
                     {:?}",
                    ranking.kind, clause.operator
                ))));
            }
            1u16
        }
        HavingRankingKind::Top | HavingRankingKind::Bottom => {
            let n = ranking.n.ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidParameter(format!(
                    "`{:?}(n)` ranking requires `n`; use `MAX` / `MIN` for the single \
                     best group",
                    ranking.kind
                )))
            })?;
            if n == 0 {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "`{:?}(0)` selects nothing; ranked queries require 1 ≤ n ≤ {}",
                    ranking.kind, MAX_RANKED_LIMIT
                ))));
            }
            if n > MAX_RANKED_LIMIT as u64 {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "`{:?}({n})` exceeds the ranked-query ceiling of {}; the proof commits \
                     one secondary entry per returned group, so its size grows linearly \
                     in n. Narrow the request — the ceiling is a hard limit, not a clamp, \
                     because `k` is echoed in the proof envelope and re-checked by the \
                     verifier.",
                    ranking.kind, MAX_RANKED_LIMIT
                ))));
            }
            // `IN TOP(n)` is the canonical membership form; `= TOP(1)` is
            // documented as equivalent to `= MAX` on `HavingRightOperand`.
            match clause.operator {
                HavingOperator::In => {}
                HavingOperator::Equal if n == 1 => {}
                other => {
                    return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                        "`{:?}(n)` ranking is supported with the `in` having operator \
                         (membership in the top/bottom set), or with `=` when n = 1; got \
                         {other:?} with n = {n}",
                        ranking.kind
                    ))));
                }
            }
            // Bounded by MAX_RANKED_LIMIT (a u16) immediately above.
            n as u16
        }
    };

    // ---- WHERE: must be absent --------------------------------------
    //
    // Rejected rather than ignored. A single-property ranked index has no
    // equality prefix a clause could narrow, and a clause on the ranked
    // property itself asks for a ranking over a filtered subset — which
    // the secondary cannot answer, because it is ordered by aggregate,
    // not by group key. Silently dropping the filter would return the
    // global ranking under the guise of a filtered one.
    if !where_clauses.is_empty() {
        return Err(Error::Query(
            QuerySyntaxError::InvalidWhereClauseComponents(
                "ranked queries do not accept `where` clauses: ranked indexes are \
                 single-property, so there is no equality prefix to narrow, and the axis \
                 secondary is ordered by aggregate rather than by group key — it cannot \
                 rank a filtered subset. Rank the whole index, or add a narrower index.",
            ),
        ));
    }

    // ---- Pagination: must be absent ---------------------------------
    if let Some(limit) = pagination.limit {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "ranked queries do not accept `limit` (got {limit}); the result size is the \
             `n` of the `having` ranking. Change `TOP(n)` / `BOTTOM(n)` instead."
        ))));
    }
    if let Some(offset) = pagination.offset {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "ranked queries do not accept `offset` (got {offset}); offset-paginated \
             ranking is a separate grovedb primitive that is not exposed yet."
        ))));
    }
    if pagination.has_start_at {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(
            "ranked queries do not accept `start_at` / `start_after`: the cursor names a \
             document id, but a ranked walk iterates an aggregate-ordered keyspace in \
             which document ids do not appear."
                .to_string(),
        )));
    }

    Ok(DocumentRankedMode {
        axis,
        descending,
        k,
        group_by_property,
        aggregate_field,
    })
}
