//! v0 of `compute_aggregate_mode_and_check_limit`.
//!
//! Original routing table extracted verbatim from
//! `query/document_query/v1/mod.rs` so the v1 cutover is a pure code
//! move with no semantic change. See the dispatcher's module-level
//! docstring for the versioning rationale.

use crate::error::query::QueryError;
use crate::query::document_query::v1::not_yet_implemented;
use drive::error::query::QuerySyntaxError;
use drive::query::{CountMode, WhereClause, WhereOperator};

pub(super) fn compute_aggregate_mode_and_check_limit_v0(
    group_by: &[String],
    where_clauses: &[WhereClause],
    limit: Option<u32>,
    function_name: &str,
) -> Result<CountMode, QueryError> {
    let is_range_op = |op: WhereOperator| {
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
    };
    let is_in_field = |field: &str| {
        where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In && wc.field == field)
    };
    let is_range_field = |field: &str| {
        where_clauses
            .iter()
            .any(|wc| is_range_op(wc.operator) && wc.field == field)
    };

    let mode = match group_by {
        [] => CountMode::Aggregate,
        [field] => {
            if is_in_field(field) {
                CountMode::GroupByIn
            } else if is_range_field(field) {
                CountMode::GroupByRange
            } else {
                return Err(not_yet_implemented(&format!(
                    "GROUP BY on field '{}' which is not constrained by an `In` or range \
                     where clause",
                    field
                )));
            }
        }
        [first, second] => {
            if is_in_field(first) && is_range_field(second) {
                CountMode::GroupByCompound
            } else {
                return Err(not_yet_implemented(
                    "two-field GROUP BY outside the `(In, range)` compound shape",
                ));
            }
        }
        _ => return Err(not_yet_implemented("GROUP BY with more than two fields")),
    };

    if limit.is_some() && !mode.accepts_limit() {
        let reason = match mode {
            CountMode::Aggregate => format!(
                "`limit` is not valid for SELECT {} with empty GROUP BY (aggregate is a \
                 single row; omit `limit` to fix)",
                function_name
            ),
            CountMode::GroupByIn => format!(
                "`limit` is not valid for SELECT {} with GROUP BY on an `In` field \
                 (result is bounded by the In array — capped at 100 entries; narrow the \
                 In array directly to reduce the result set)",
                function_name
            ),
            CountMode::GroupByRange | CountMode::GroupByCompound => unreachable!(
                "`accepts_limit()` returns true for these variants; outer guard already \
                 filtered them out"
            ),
        };
        return Err(QueryError::Query(QuerySyntaxError::InvalidLimit(reason)));
    }

    Ok(mode)
}
