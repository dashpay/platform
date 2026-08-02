//! v1 of `compute_aggregate_mode_and_check_limit` — the protocol
//! version 14 routing table, which adds the ranked (`HAVING …
//! TOP(n)`) path.
//!
//! Behaviour splits on `having`:
//!
//! - **empty** — delegate to [`v0`](super::v0) verbatim. Every
//!   non-ranked request routes exactly as it did before the upgrade,
//!   which is what makes v1 safe to activate: only requests a v13 node
//!   would have *rejected* behave differently.
//! - **non-empty** — validate the routing-level shape and hand the
//!   request to the ranked executor.
//!
//! ## What "routing-level shape" means here
//!
//! This function answers one question — *is this request bound for the
//! ranked executor?* — and nothing else. The full ranked grammar
//! (which axis, which direction, `1 ≤ n ≤ 100`, `having` aggregate
//! byte-equal to the `select`, no `where`, no `limit` / `offset` /
//! `start_at`, exactly one `group_by` property, and whether the
//! contract's index actually declares the axis) is owned by drive's
//! `drive_document_ranked_query::mode_detection::detect_ranked_mode`
//! and its index picker, and is itself versioned there.
//!
//! Re-validating any of that here would create two sources of truth
//! for one grammar: the SDK's proof helpers call drive's validator
//! directly (no abci in the path), so a routing-layer copy that
//! drifted would make the server accept or reject requests the client
//! disagrees about. The three checks below are the minimum needed to
//! *route*:
//!
//! 1. **Exactly one `having` clause.** Multiple clauses are implicitly
//!    ANDed, and "in the top 5 by X and the bottom 3 by Y" is not a
//!    ranked query — it is an intersection with no storage primitive.
//!    With two clauses there is no single ranking to route on.
//! 2. **The right operand is a ranking.** A `HAVING <agg> > <value>`
//!    threshold is a different (unimplemented) feature, not a
//!    misspelled ranking, so it keeps the `not_yet_implemented`
//!    contract rather than becoming an argument error.
//! 3. **`group_by` is non-empty.** A ranking ranks groups; with no
//!    grouping there is exactly one implicit group and the request is
//!    structurally meaningless rather than merely unsupported. Drive
//!    would reject it too, but routing a request that cannot succeed
//!    would cost a contract fetch first.
//!
//! Note what is *not* checked: `k`'s bounds, the select↔having
//! agreement, `where` / `limit` / `start_at` presence. Those all reach
//! drive and come back as `Error::Query(...)`, which the ranked
//! dispatcher maps onto `QueryError::Query` — a client-visible
//! rejection, not an internal error.

use super::v0::compute_aggregate_mode_and_check_limit_v0;
use super::AggregateRouting;
use crate::error::query::QueryError;
use crate::query::document_query::v1::not_yet_implemented;
use drive::error::query::QuerySyntaxError;
use drive::query::{HavingClause, HavingRightOperand, WhereClause};

pub(super) fn compute_aggregate_mode_and_check_limit_v1(
    group_by: &[String],
    where_clauses: &[WhereClause],
    limit: Option<u32>,
    having: &[HavingClause],
    function_name: &str,
) -> Result<AggregateRouting, QueryError> {
    let clause = match having {
        // No HAVING: identical routing to v0, including its
        // `accepts_limit()` enforcement. `having` is passed through as
        // the empty slice it is, so v0's own blanket rejection is
        // trivially satisfied.
        [] => {
            return compute_aggregate_mode_and_check_limit_v0(
                group_by,
                where_clauses,
                limit,
                having,
                function_name,
            )
        }
        [only] => only,
        _ => {
            return Err(not_yet_implemented(&format!(
                "SELECT {} with {} HAVING clauses (ranked queries take exactly one \
                 clause carrying the ranking; multiple HAVING clauses are implicitly \
                 ANDed and there is no ranked primitive for an intersection of two \
                 rankings)",
                function_name,
                having.len()
            )));
        }
    };

    match clause.right {
        HavingRightOperand::Ranking(_) => {}
        HavingRightOperand::Value(_) => {
            return Err(not_yet_implemented(&format!(
                "HAVING {}(…) <operator> <value> — a threshold comparison on an \
                 aggregate (the wire surface accepts a literal right operand so \
                 callers can encode it ahead of server support landing, but today \
                 only a ranking right operand — TOP(n) / BOTTOM(n) / MAX / MIN — is \
                 evaluated)",
                function_name
            )));
        }
    }

    if group_by.is_empty() {
        return Err(QueryError::Query(QuerySyntaxError::InvalidParameter(
            format!(
                "HAVING {}(…) with a ranking operand requires a GROUP BY property: a \
                 ranking orders groups against each other, and with no GROUP BY there \
                 is a single implicit group with nothing to rank it against. Add \
                 `group_by` naming the ranked index's property.",
                function_name
            ),
        )));
    }

    Ok(AggregateRouting::Ranked)
}
