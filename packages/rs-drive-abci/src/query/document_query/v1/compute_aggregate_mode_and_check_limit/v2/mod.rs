//! Feature version 2 of the aggregate routing helper: the boolean-`HAVING`
//! range activation.
//!
//! Differs from v1 in exactly one branch: a grouped aggregate carrying a
//! **single** `having` clause routes to the having-range executor
//! ([`AggregateRouting::HavingRange`]) instead of being refused. Routing
//! here only asks "is there a grouped select with exactly one having
//! clause?" — whether the clause bounds the selected aggregate, whether
//! its operator translates to a contiguous range, whether an `order_by`
//! is compatible, and the limit's bounds are all drive's call, made in
//! `detect_having_mode`; a routing-layer copy of that grammar could
//! disagree with drive's after a version bump.
//!
//! Multi-clause `having` (implicit AND) keeps the `not_yet_implemented`
//! contract: each extra clause needs a per-candidate post-check against
//! the primary that no executor performs yet. And `having` without
//! `group_by` still falls through to the v1 → v0 blanket rejection — a
//! global aggregate produces one row, and bounding it is a client-side
//! comparison, not a query.

use super::v1::compute_aggregate_mode_and_check_limit_v1;
use super::AggregateRouting;
use crate::error::query::QueryError;
use crate::query::document_query::v1::not_yet_implemented;
use drive::query::{HavingClause, OrderClause, SelectProjection, WhereClause};

#[allow(clippy::too_many_arguments)]
pub(super) fn compute_aggregate_mode_and_check_limit_v2(
    select: &SelectProjection,
    group_by: &[String],
    where_clauses: &[WhereClause],
    order_by: &[OrderClause],
    limit: Option<u32>,
    having: &[HavingClause],
    function_name: &str,
) -> Result<AggregateRouting, QueryError> {
    if !having.is_empty() && !group_by.is_empty() {
        return match having {
            [_single] => Ok(AggregateRouting::HavingRange),
            many => Err(not_yet_implemented(&format!(
                "multiple HAVING clauses (implicit AND): got {}. One clause on the \
                 selected {function_name} aggregate is served as a single contiguous \
                 range read of the covering ranked index's axis secondary; additional \
                 clauses would need a per-candidate post-check that is not implemented. \
                 Narrow to a single clause",
                many.len()
            ))),
        };
    }

    // No having (or no group_by, where a having still dies in v0's
    // blanket rejection): identical routing to v1, including its ranked
    // detection and its delegation to v0 for non-ranked shapes.
    compute_aggregate_mode_and_check_limit_v1(
        select,
        group_by,
        where_clauses,
        order_by,
        limit,
        having,
        function_name,
    )
}
