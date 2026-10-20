//! v1 of `compute_aggregate_mode_and_check_limit` — the protocol
//! version 14 routing table, which adds the ranked
//! (`ORDER BY <aggregate> LIMIT n OFFSET m`) path.
//!
//! Behaviour splits on the **`order_by` clause**:
//!
//! - the request names the selected aggregate in a single `ORDER BY`
//!   clause, and has a `GROUP BY` — route to the ranked executor.
//! - anything else — delegate to [`v0`](super::v0) verbatim. Every
//!   non-ranked request routes exactly as it did before the upgrade,
//!   which is what makes v1 safe to activate: only requests a v13 node
//!   would have *rejected* behave differently.
//!
//! ## What "routing-level shape" means here
//!
//! This function answers one question — *is this request bound for the
//! ranked executor?* — and nothing else. The full ranked grammar
//! (which axis, which direction, `1 ≤ limit ≤ 100`, no `where`, no
//! `having`, no `start_at`, exactly one `group_by` property, and
//! whether the contract's index actually declares the axis) is owned by
//! drive's
//! `drive_document_ranked_query::mode_detection::detect_ranked_mode`
//! and its index picker, and is itself versioned there.
//!
//! Re-validating any of that here would create two sources of truth
//! for one grammar: the SDK's proof helpers call drive's validator
//! directly (no abci in the path), so a routing-layer copy that
//! drifted would make the server accept or reject requests the client
//! disagrees about. The checks below are the minimum needed to *route*:
//!
//! 1. **`group_by` is non-empty.** A ranking ranks groups; with no
//!    grouping there is exactly one implicit group and `ORDER BY
//!    <aggregate>` is a no-op rather than a ranking.
//! 2. **Exactly one `order_by` clause, naming the selected
//!    aggregate.** This is the discriminator, and it has to be exact:
//!    `SELECT COUNT(*) GROUP BY brand ORDER BY brand` is a perfectly
//!    ordinary grouped count and must keep routing to the grouped
//!    path, while `… ORDER BY $count` is the ranked one. Matching on
//!    the field name is what keeps those two apart.
//! 3. **`having` is empty on the ranked path.** Boolean per-group
//!    predicates are not evaluated at all yet, and combining one with
//!    an aggregate ordering is a distinct (also unimplemented)
//!    capability — see the rejection below.
//!
//! Note what is *not* checked: the limit's bounds, its presence at
//! all, `where` / `start_at` presence, `offset`'s value. Those all
//! reach drive and come back as `Error::Query(...)`, which the ranked
//! dispatcher maps onto `QueryError::Query` — a client-visible
//! rejection, not an internal error.

use super::v0::compute_aggregate_mode_and_check_limit_v0;
use super::AggregateRouting;
use crate::error::query::QueryError;
use crate::query::document_query::v1::not_yet_implemented;
use drive::query::{
    HavingClause, OrderClause, SelectProjection, WhereClause, RANKED_COUNT_ORDER_KEY,
};

/// The `order_by` field name that names `select`'s aggregate:
/// the projection's own field for `SUM(f)` / `AVG(f)`, and the
/// [`RANKED_COUNT_ORDER_KEY`] sentinel for `COUNT(*)`.
///
/// Mirrors drive's private `ranked_order_key`. The duplication is
/// deliberate and safe in a way the rest of the grammar would not be:
/// this is a *name*, pinned by a shared public constant, not a rule.
/// A drift would be a compile-time constant mismatch, not a silent
/// semantic disagreement — and calling into drive here would mean
/// building a `DocumentRankedMode` before the contract has even been
/// fetched.
fn aggregate_order_key(select: &SelectProjection) -> &str {
    if select.field.is_empty() {
        RANKED_COUNT_ORDER_KEY
    } else {
        select.field.as_str()
    }
}

pub(super) fn compute_aggregate_mode_and_check_limit_v1(
    select: &SelectProjection,
    group_by: &[String],
    where_clauses: &[WhereClause],
    order_by: &[OrderClause],
    limit: Option<u32>,
    having: &[HavingClause],
    function_name: &str,
) -> Result<AggregateRouting, QueryError> {
    let orders_by_the_aggregate = match order_by {
        [only] => only.field == aggregate_order_key(select),
        _ => false,
    };

    if group_by.is_empty() || !orders_by_the_aggregate {
        // Not a ranked shape: identical routing to v0, including its
        // `accepts_limit()` enforcement and its blanket HAVING
        // rejection.
        return compute_aggregate_mode_and_check_limit_v0(
            group_by,
            where_clauses,
            limit,
            having,
            function_name,
        );
    }

    // Ranked shape. `having` is refused here rather than downstream
    // because the two rejections say different things: drive's is
    // about the ranked *executor* and only fires once a contract has
    // been fetched, while this one keeps the `not_yet_implemented`
    // contract the rest of the v1 surface uses for capabilities that
    // are wired on the wire but not in the server.
    if !having.is_empty() {
        return Err(not_yet_implemented(&format!(
            "boolean HAVING combined with `ORDER BY {}(…)` (ranking reads a pre-sorted \
             axis secondary, so filtering groups first would mean walking every group to \
             test the predicate — the exact cost the ranked surface exists to avoid)",
            function_name
        )));
    }

    Ok(AggregateRouting::Ranked)
}
