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
//!
//! Two public surfaces, both routed through the same method-version
//! slot to guarantee server + SDK pick the same executor, and the
//! doctype cross-validation both run:
//!
//! - [`detect_sum_mode`] (server-side) — takes a full
//!   [`DocumentSumRequest`] and additionally cross-validates the
//!   request's `sum_property` against the doctype's
//!   `documents_summable`. Server-only because
//!   [`DocumentSumRequest`] is `#[cfg(feature = "server")]`.
//!
//! - [`detect_sum_mode_from_inputs`] — takes the minimal
//!   `(where_clauses, mode, prove)` tuple the routing table
//!   actually needs, callable from both server and SDK (`server`
//!   OR `verify` features). Mirrors count's
//!   [`crate::query::drive_document_count_query::DriveDocumentCountQuery::detect_mode_versioned`]
//!   so the SDK's verify path picks the same executor the prover
//!   used — without this, the SDK had to reconstruct routing from
//!   ad-hoc `(has_range, has_in, distinct_mode)` booleans, which
//!   could disagree with the v0 routing table on edge cases (e.g.
//!   `group_by = [in_field]` with a co-present range clause routes
//!   to the carrier shape server-side but the heuristic would have
//!   sent it to an aggregate verifier).
//!
//! - [`check_sum_property_against_document_type`] — the doctype
//!   cross-validation alone, which [`detect_sum_mode`] runs first and
//!   a client runs before sending.

#[cfg(any(feature = "server", feature = "verify"))]
mod v0;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::query::drive_document_sum_query::DocumentSumRequest;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::query::drive_document_sum_query::{DocumentSumMode, SumMode};
#[cfg(any(feature = "server", feature = "verify"))]
use crate::query::WhereClause;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

/// Server-side wrapper around [`detect_sum_mode_from_inputs`] that
/// adds the doctype cross-validation (`sum_property` must agree
/// with the doctype's `documents_summable`,
/// [`check_sum_property_against_document_type`]). Server-only because
/// [`DocumentSumRequest`] is gated behind the `server` feature.
///
/// Routes through
/// `platform_version.drive.methods.document.query.detect_sum_mode`.
#[cfg(feature = "server")]
pub fn detect_sum_mode(
    request: &DocumentSumRequest,
    platform_version: &PlatformVersion,
) -> Result<DocumentSumMode, Error> {
    // Clients run the same check before sending. It reads no version table, so it refuses
    // the same requests with the same error at every protocol version.
    check_sum_property_against_document_type(request.document_type, &request.sum_property)?;

    detect_sum_mode_from_inputs(
        &request.where_clauses,
        request.mode,
        request.prove,
        platform_version,
    )
}

/// Refuse a sum of `sum_property` over a document type whose documents are summed by
/// another property (`documentsSummable`), whatever the where clauses and indexes.
///
/// [`detect_sum_mode`] refuses such a request with this error before it picks an index, so
/// every node refuses it alike; a client calls this before sending to refuse the same
/// requests.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn check_sum_property_against_document_type(
    document_type: DocumentTypeRef,
    sum_property: &str,
) -> Result<(), Error> {
    if let Some(doctype_sum) = document_type.documents_summable() {
        if doctype_sum != sum_property {
            return Err(Error::Drive(DriveError::NotSupported(
                "request `sum_property` doesn't match the document type's \
                 `documents_summable`. Sum trees aggregate `i64` per merk node; \
                 mixing property names would produce a meaningless aggregation. \
                 Define a separate index whose `summable: \"<the other name>\"` \
                 covers the alternate aggregation surface.",
            )));
        }
    }
    Ok(())
}

/// Same routing decision as [`detect_sum_mode`] but takes the
/// minimal `(where_clauses, mode, prove)` tuple — callable from
/// SDK verify paths that don't have a `DocumentSumRequest`.
///
/// Skips the `sum_property ↔ documents_summable` cross-validation
/// that the server does; a client runs it before sending with
/// [`check_sum_property_against_document_type`].
///
/// Routes through the same method-version slot as
/// [`detect_sum_mode`] so server + SDK can never drift.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn detect_sum_mode_from_inputs(
    where_clauses: &[WhereClause],
    mode: SumMode,
    prove: bool,
    platform_version: &PlatformVersion,
) -> Result<DocumentSumMode, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .detect_sum_mode
    {
        0 => v0::detect_sum_mode_v0_from_inputs(where_clauses, mode, prove),
        version => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "detect_sum_mode: unknown method version {version}; only 0 is supported"
        )))),
    }
}
