//! Versioned grouping of a query's raw where clauses into
//! `(equal_clauses, range_clause, in_clauses)`.
//!
//! The error surface for rejected shapes is part of the query contract,
//! so the grouping dispatches on
//! `platform_version.drive.methods.document.query.where_clause_grouping`:
//! v0 (protocol versions up to 13) rejects any query with more than one
//! non-primary-key `In` clause with `MultipleInClauses` before any other
//! same-field checks; v1 (protocol version 14) groups multiple `In`
//! clauses structurally, leaving their acceptance to the versioned
//! path-query lowering.

mod v0;
mod v1;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::conditions::WhereClause;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// `(equal_clauses, range_clause, in_clauses)`.
pub(crate) type GroupedWhereClauses = (
    BTreeMap<String, WhereClause>,
    Option<WhereClause>,
    Vec<WhereClause>,
);

/// Group raw where clauses under the platform version's grammar.
pub(crate) fn group_where_clauses(
    where_clauses: &[WhereClause],
    platform_version: &PlatformVersion,
) -> Result<GroupedWhereClauses, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .where_clause_grouping
    {
        0 => {
            let (equal_clauses, range_clause, in_clause) =
                v0::group_where_clauses_v0(where_clauses)?;
            Ok((
                equal_clauses,
                range_clause,
                in_clause.map_or_else(Vec::new, |in_clause| vec![in_clause]),
            ))
        }
        1 => v1::group_where_clauses_v1(where_clauses),
        version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method: "group_where_clauses".to_string(),
            known_versions: vec![0, 1],
            received: version,
        })),
    }
}
