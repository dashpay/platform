//! Versioned dispatcher for the
//! `compute_aggregate_mode_and_check_limit` helper.
//!
//! The helper picks the `(group_by × where)` execution mode for
//! `SELECT COUNT` / `SUM` / `AVG` against the v1 query surface, then
//! enforces the per-mode `accepts_limit()` contract. The routing rules
//! it embeds are part of the query contract clients see on the wire —
//! a future change to which `(group_by × where_clauses)` shapes are
//! accepted (e.g. adding a third group_by field, or relaxing the
//! "Aggregate rejects limit" rule) becomes consensus-visible because
//! the dispatcher runs on every v1 query request. Versioning it lets
//! later protocol bumps adjust the routing table without breaking
//! older nodes' replay of historical traffic.
//!
//! Lives next to the v1 query handler (the only call site today) and
//! is dispatched via the `DriveAbciDocumentQueryHelperVersions` slot
//! in `PlatformVersion`.

mod v0;

use crate::error::query::QueryError;
use dpp::version::PlatformVersion;
use drive::query::{CountMode, WhereClause};

/// Compute the `(group_by × where)` mode for SELECT COUNT / SUM / AVG.
///
/// All three aggregate functions share the same SQL-shape contract
/// (empty group_by → Aggregate; one-field group_by → GroupByIn or
/// GroupByRange depending on whether the field is `In`-bound or
/// range-bound; two-field group_by `(in_field, range_field)` →
/// GroupByCompound). The `function_name` arg ("COUNT" / "SUM" / "AVG")
/// is woven into rejection messages for clarity.
///
/// Also runs the `accepts_limit()` check: `Aggregate` and `GroupByIn`
/// can't honor a caller-supplied limit; rejects with
/// `QuerySyntaxError::InvalidLimit` if one is set.
///
/// Routes through `platform_version.drive_abci.query.document_query_helpers.compute_aggregate_mode_and_check_limit`.
pub(super) fn compute_aggregate_mode_and_check_limit(
    group_by: &[String],
    where_clauses: &[WhereClause],
    limit: Option<u32>,
    function_name: &str,
    platform_version: &PlatformVersion,
) -> Result<CountMode, QueryError> {
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
            function_name,
        ),
        version => Err(QueryError::Drive(drive::error::Error::Drive(
            drive::error::drive::DriveError::UnknownVersionMismatch {
                method: "compute_aggregate_mode_and_check_limit".to_string(),
                known_versions: vec![0],
                received: version,
            },
        ))),
    }
}
