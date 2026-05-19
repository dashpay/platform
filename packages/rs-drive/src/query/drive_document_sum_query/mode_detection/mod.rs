//! Versioned mode detection for the sum-query dispatcher.
//!
//! `detect_sum_mode` classifies a [`DocumentSumRequest`] into one of
//! the [`DocumentSumMode`] variants by inspecting the
//! `(where × SumMode × prove)` triple. The result picks the
//! executor the dispatcher routes to.
//!
//! Versioned because the routing table is a consensus-relevant
//! contract on the query surface — a future protocol version that
//! adds or relaxes shapes (e.g. a new "GroupByRange + In + prove"
//! mapping) has to land behind a method-version bump so older
//! nodes replaying historical traffic keep dispatching the way
//! the chain originally saw.

mod v0;

use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_sum_query::{DocumentSumMode, DocumentSumRequest};
use dpp::version::PlatformVersion;

/// Determine which executor to dispatch to based on the request's
/// where-shape × mode × prove combination. Pure function; no I/O.
///
/// Returns `Err(WhereClauseOnNonIndexedProperty)` (via
/// [`QuerySyntaxError`]) if no covering index can be found — same
/// strict-coverage contract count uses, with the addition that
/// the request's `sum_property` must match the chosen index's
/// `summable` declaration.
///
/// Routes through
/// `platform_version.drive.methods.document.query.detect_sum_mode`.
pub fn detect_sum_mode(
    request: &DocumentSumRequest,
    platform_version: &PlatformVersion,
) -> Result<DocumentSumMode, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .detect_sum_mode
    {
        0 => v0::detect_sum_mode_v0(request),
        version => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "detect_sum_mode: unknown method version {version}; only 0 is supported"
        )))),
    }
}
