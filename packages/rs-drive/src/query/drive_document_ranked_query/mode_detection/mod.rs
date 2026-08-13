//! Request-shape validation for the ranked query, and the versioned
//! `(select, group_by, order_by, limit, offset)` → [`DocumentRankedMode`]
//! resolution.
//!
//! Pure functions on the request shape — no Drive, no contract, no
//! indexes. Available under `server` (the dispatcher validates before
//! executing) and `verify` (the SDK validates the same way before
//! attempting proof verification), so both sides agree on which requests
//! are well-formed and on the `(axis, descending, k, offset)` tuple a
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

use super::{
    DocumentRankedMode, RankedAxis, RankedPaginationInputs, MAX_RANKED_LIMIT,
    RANKED_COUNT_ORDER_KEY,
};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::HavingClause;
use crate::query::projection::{SelectFunction, SelectProjection};
use crate::query::{OrderClause, WhereClause};
use dpp::version::PlatformVersion;

/// Versioned entry point. Routes through
/// `platform_version.drive.methods.document.query.detect_ranked_mode`;
/// today only `0` is defined and maps to [`detect_ranked_mode_v0`]
/// verbatim.
pub fn detect_ranked_mode(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    order_by: &[OrderClause],
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
        0 => detect_ranked_mode_v0(
            select,
            group_by,
            having,
            order_by,
            where_clauses,
            pagination,
        ),
        version => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "detect_ranked_mode: unknown method version {version}; only 0 is supported"
        )))),
    }
}

/// The `ORDER BY` field name that names a given select's aggregate.
///
/// `SUM(f)` / `AVG(f)` are ordered by naming `f` — the same field the
/// projection aggregates, which is how SQL's `ORDER BY avg(grade)`
/// reads once the aggregate function is already fixed by the `SELECT`.
/// `COUNT(*)` has no field, so it is named by the
/// [`RANKED_COUNT_ORDER_KEY`] sentinel.
///
/// Public because request *builders* need it as much as the validator
/// does: an SDK offering `.order_by_selected_aggregate(…)` has to emit
/// the same string this function expects to read back, and a second
/// copy of the sentinel rule is a silent-rejection bug waiting for the
/// first `COUNT(*)` ranking.
pub fn ranked_order_key(select: &SelectProjection) -> &str {
    match select.function {
        SelectFunction::Count if select.field.is_empty() => RANKED_COUNT_ORDER_KEY,
        _ => select.field.as_str(),
    }
}

mod v0;
// Re-exported so the dispatcher's callers (`drive_dispatcher`, the
// test suites) keep addressing the frozen grammar by its old path.
pub use v0::{detect_ranked_mode_v0, equality_pins_from_where_clauses};
