//! Versioned dispatcher for the
//! `compute_aggregate_mode_and_check_limit` helper.
//!
//! The helper decides how a `SELECT COUNT` / `SUM` / `AVG` request
//! against the v1 query surface is executed: as one of the four
//! `(group_by × where)` grouped modes, or — from feature version 1 —
//! as a *ranked* request when the request orders by the aggregate it
//! selects (`GROUP BY p ORDER BY <the selected aggregate>`), or — from
//! feature version 2 — as a *having-range* request when a grouped
//! aggregate carries exactly one `having` clause. Routing here only
//! asks which shape the request has; the direction, the bounds, the
//! limit's contract and the offset are drive's call and are made in
//! `detect_ranked_mode` / `detect_having_mode`. It also enforces the
//! per-mode `accepts_limit()` contract on the grouped path.
//!
//! The routing rules it embeds are part of the query contract clients
//! see on the wire — a change to which `(group_by × where_clauses ×
//! order_by)` shapes are accepted becomes consensus-visible because the
//! dispatcher runs on every v1 query request. Versioning it lets later
//! protocol bumps adjust the routing table without breaking older
//! nodes' replay of historical traffic, and is what keeps a
//! mixed-version network in agreement across the ranked-query and
//! having-range activations: protocol version 13 and earlier select v0,
//! which has neither path, while protocol version 14 selects v2 and
//! answers both.
//!
//! Lives next to the v1 query handler (the only call site today) and
//! is dispatched via the `DriveAbciDocumentQueryHelperVersions` slot
//! in `PlatformVersion`.

mod v0;
mod v1;
mod v2;

use crate::error::query::QueryError;
use dpp::version::PlatformVersion;
use drive::query::{CountMode, HavingClause, OrderClause, SelectProjection, WhereClause};

/// What the helper decided a request should execute as.
///
/// `Grouped` is the pre-existing outcome — the `(group_by × where)`
/// mode the count / sum / average executors take. `Ranked` is the
/// feature-version-1 addition: the request orders by the aggregate it
/// selects and belongs to `Drive::execute_document_ranked_request`,
/// which re-derives `(axis, direction, k, offset)` from the same
/// `(select, group_by, order_by, limit, offset)` inputs.
///
/// The ranked variant deliberately carries no data. Everything the
/// ranked dispatcher needs is already owned by the handler (the
/// decoded `select`, `group_by`, `order_by`, `limit` and `offset`), and
/// duplicating the resolved axis / direction / `k` here would create a
/// second place where the ranking grammar is interpreted — drive owns
/// that grammar (`detect_ranked_mode`), and a routing-layer copy could
/// disagree with it after a version bump. Routing decides *where* a
/// request goes; drive decides what it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AggregateRouting {
    /// Non-ranked aggregate: execute with this `(group_by × where)` mode.
    Grouped(CountMode),
    /// Ranked aggregate: execute through the ranked (top-k) surface.
    Ranked,
    /// Boolean-`HAVING` range: a grouped aggregate carrying exactly one
    /// `having` clause, executed through
    /// `Drive::execute_document_having_request` as a value-bounded range
    /// read of the covering ranked index's axis secondary. The
    /// feature-version-2 addition. Carries no data for the same reason
    /// `Ranked` carries none: drive owns the having grammar
    /// (`detect_having_mode`), and routing only decides *where* the
    /// request goes.
    HavingRange,
}

/// Decide how a `SELECT COUNT` / `SUM` / `AVG` request executes.
///
/// All three aggregate functions share the same SQL-shape contract
/// (empty group_by → Aggregate; one-field group_by → GroupByIn or
/// GroupByRange depending on whether the field is `In`-bound or
/// range-bound; two-field group_by `(in_field, range_field)` →
/// GroupByCompound). The `function_name` arg ("COUNT" / "SUM" / "AVG")
/// is woven into rejection messages for clarity.
///
/// On the grouped path it also runs the `accepts_limit()` check:
/// `Aggregate` and `GroupByIn` can't honor a caller-supplied limit and
/// reject with `QuerySyntaxError::InvalidLimit` if one is set. The
/// ranked path deliberately does *not* check `limit` here — on that
/// path `limit` is the ranking's `k`, and drive owns both its
/// mandatoriness and its `1 ..= 100` bound; duplicating those rules at
/// the routing layer would mean two error messages for one contract.
///
/// Routes through `platform_version.drive_abci.query.document_query_helpers.compute_aggregate_mode_and_check_limit`.
#[allow(clippy::too_many_arguments)]
pub(super) fn compute_aggregate_mode_and_check_limit(
    select: &SelectProjection,
    group_by: &[String],
    where_clauses: &[WhereClause],
    order_by: &[OrderClause],
    limit: Option<u32>,
    having: &[HavingClause],
    function_name: &str,
    platform_version: &PlatformVersion,
) -> Result<AggregateRouting, QueryError> {
    match platform_version
        .drive_abci
        .query
        .document_query_helpers
        .compute_aggregate_mode_and_check_limit
    {
        // v0 predates the ranked surface entirely: it has no `order_by`
        // input because ordering played no part in its routing
        // decision, and passing one would be the first step toward
        // changing what an already-shipped protocol version accepts.
        0 => v0::compute_aggregate_mode_and_check_limit_v0(
            group_by,
            where_clauses,
            limit,
            having,
            function_name,
        ),
        1 => v1::compute_aggregate_mode_and_check_limit_v1(
            select,
            group_by,
            where_clauses,
            order_by,
            limit,
            having,
            function_name,
        ),
        2 => v2::compute_aggregate_mode_and_check_limit_v2(
            select,
            group_by,
            where_clauses,
            order_by,
            limit,
            having,
            function_name,
        ),
        version => Err(QueryError::Drive(drive::error::Error::Drive(
            drive::error::drive::DriveError::UnknownVersionMismatch {
                method: "compute_aggregate_mode_and_check_limit".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            },
        ))),
    }
}
