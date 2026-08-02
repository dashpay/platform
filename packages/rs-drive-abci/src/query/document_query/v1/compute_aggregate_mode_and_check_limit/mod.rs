//! Versioned dispatcher for the
//! `compute_aggregate_mode_and_check_limit` helper.
//!
//! The helper decides how a `SELECT COUNT` / `SUM` / `AVG` request
//! against the v1 query surface is executed: as one of the four
//! `(group_by × where)` grouped modes, or — from feature version 1 —
//! as a *ranked* request when a `HAVING … TOP(n)` / `BOTTOM(n)` /
//! `MAX` / `MIN` clause is present. It also enforces the per-mode
//! `accepts_limit()` contract on the grouped path.
//!
//! The routing rules it embeds are part of the query contract clients
//! see on the wire — a change to which `(group_by × where_clauses ×
//! having)` shapes are accepted becomes consensus-visible because the
//! dispatcher runs on every v1 query request. Versioning it lets later
//! protocol bumps adjust the routing table without breaking older
//! nodes' replay of historical traffic, and is what keeps a
//! mixed-version network in agreement across the ranked-query
//! activation: protocol version 13 and earlier select v0 and keep
//! rejecting every non-empty `having`, while protocol version 14
//! selects v1 and answers ranked queries.
//!
//! Lives next to the v1 query handler (the only call site today) and
//! is dispatched via the `DriveAbciDocumentQueryHelperVersions` slot
//! in `PlatformVersion`.

mod v0;
mod v1;

use crate::error::query::QueryError;
use dpp::version::PlatformVersion;
use drive::query::{CountMode, HavingClause, WhereClause};

/// What the helper decided a request should execute as.
///
/// `Grouped` is the pre-existing outcome — the `(group_by × where)`
/// mode the count / sum / average executors take. `Ranked` is the
/// feature-version-1 addition: the request carries a ranking `having`
/// clause and belongs to `Drive::execute_document_ranked_request`,
/// which re-derives `(axis, direction, k)` from the same
/// `(select, group_by, having)` triple.
///
/// The ranked variant deliberately carries no data. Everything the
/// ranked dispatcher needs is already owned by the handler (the
/// decoded `select`, `group_by` and `having`), and duplicating the
/// resolved axis / direction / `k` here would create a second place
/// where the ranking grammar is interpreted — drive owns that grammar
/// (`detect_ranked_mode`), and a routing-layer copy could disagree
/// with it after a version bump. Routing decides *where* a request
/// goes; drive decides what it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AggregateRouting {
    /// Non-ranked aggregate: execute with this `(group_by × where)` mode.
    Grouped(CountMode),
    /// Ranked aggregate: execute through the ranked (top-k) surface.
    Ranked,
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
/// ranked path deliberately does *not* check `limit` here — drive
/// rejects `limit` on a ranked request with a message explaining that
/// the result size comes from the ranking's `n`, and duplicating that
/// rule at the routing layer would mean two error messages for one
/// contract.
///
/// Routes through `platform_version.drive_abci.query.document_query_helpers.compute_aggregate_mode_and_check_limit`.
pub(super) fn compute_aggregate_mode_and_check_limit(
    group_by: &[String],
    where_clauses: &[WhereClause],
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
        0 => v0::compute_aggregate_mode_and_check_limit_v0(
            group_by,
            where_clauses,
            limit,
            having,
            function_name,
        ),
        1 => v1::compute_aggregate_mode_and_check_limit_v1(
            group_by,
            where_clauses,
            limit,
            having,
            function_name,
        ),
        version => Err(QueryError::Drive(drive::error::Error::Drive(
            drive::error::drive::DriveError::UnknownVersionMismatch {
                method: "compute_aggregate_mode_and_check_limit".to_string(),
                known_versions: vec![0, 1],
                received: version,
            },
        ))),
    }
}
