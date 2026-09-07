//! Request-shape validation for the having-range query, and the versioned
//! `(select, group_by, having, order_by, limit)` → [`DocumentHavingMode`]
//! resolution — including the operator → inclusive-bounds translation
//! that turns a `HAVING <agg> <op> <value>` clause into an
//! [`AxisRangeBounds`].
//!
//! Pure functions on the request shape — no Drive, no contract, no
//! indexes. Available under `server` and `verify` for the same reason as
//! [`super::super::drive_document_ranked_query::mode_detection`]: both
//! sides must agree on which requests are well-formed and on the exact
//! bounds a well-formed one resolves to, because the verifier rebuilds
//! the bounded traversal from those bounds and re-executes the proof
//! against it.
//!
//! Versioned through
//! `platform_version.drive.methods.document.query.detect_having_mode` —
//! the accepted grammar is a consensus-adjacent contract on the query
//! surface, so relaxing it later (multi-clause `HAVING`, `IN`, a
//! pagination cursor) lands behind a method-version bump.

use super::super::drive_document_ranked_query::RankedPaginationInputs;
use super::{AxisRangeBounds, DocumentHavingMode, MAX_HAVING_LIMIT};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::HavingClause;
use crate::query::projection::SelectProjection;
use crate::query::{OrderClause, WhereClause};
use dpp::version::PlatformVersion;

/// Versioned entry point. Routes through
/// `platform_version.drive.methods.document.query.detect_having_mode`;
/// today only `0` is defined and maps to [`detect_having_mode_v0`]
/// verbatim.
#[allow(clippy::too_many_arguments)]
pub fn detect_having_mode(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    order_by: &[OrderClause],
    where_clauses: &[WhereClause],
    pagination: RankedPaginationInputs,
    platform_version: &PlatformVersion,
) -> Result<DocumentHavingMode, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .detect_having_mode
    {
        0 => detect_having_mode_v0(
            select,
            group_by,
            having,
            order_by,
            where_clauses,
            pagination,
        ),
        version => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "detect_having_mode: unknown method version {version}; only 0 is supported"
        )))),
    }
}

mod v0;
// Re-exported so the dispatcher's callers (`drive_dispatcher`, the
// test suites) keep addressing the frozen grammar by its old path.
pub use v0::detect_having_mode_v0;
