//! Wire-protobuf → drive type conversions for the v1 document
//! query surface.
//!
//! The decode logic itself lives in
//! `platform_query_wire::proto_conversions`, shared with client-side
//! proof verifiers so the bytes the server decodes and the bytes a
//! verifier decodes cannot drift. This module only maps the shared
//! crate's neutral [`DecodeError`] onto this crate's [`QueryError`]
//! surface:
//! - [`DecodeError::InvalidArgument`] (malformed wire input) →
//!   [`QueryError::InvalidArgument`]. The v1 handler distinguishes
//!   this from [`QuerySyntaxError::Unsupported`] (valid request
//!   shape, server capability not yet wired) — see `v1/mod.rs`'s
//!   `not_yet_implemented` helper.
//! - [`DecodeError::Unsupported`] (well-formed shape the decoder
//!   deliberately refuses, e.g. `ORDER BY` on aggregate keys) →
//!   [`QueryError::Query`]\([`QuerySyntaxError::Unsupported`]\).
//!
//! Both mappings preserve the exact message strings this module
//! produced when it owned the decode logic, so the server's error
//! surface is unchanged.

use crate::error::query::QueryError;
use dapi_grpc::platform::v0::get_documents_request::{
    get_documents_request_v1::Select as ProtoSelect, HavingClause as ProtoHavingClause,
    OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
};
use drive::error::query::QuerySyntaxError;
use drive::query::{HavingClause, OrderClause, SelectProjection, WhereClause};
use platform_query_wire::proto_conversions::{self as shared, DecodeError};

fn map_decode_error(error: DecodeError) -> QueryError {
    match error {
        DecodeError::InvalidArgument(msg) => QueryError::InvalidArgument(msg),
        DecodeError::Unsupported(msg) => QueryError::Query(QuerySyntaxError::Unsupported(msg)),
    }
}

/// Decode the request-level `repeated WhereClause` field via the
/// shared decoder. Returns an error on the first malformed clause;
/// the v1 handler surfaces this through
/// `QueryValidationResult::new_with_error` so the caller sees the
/// rejection on the same response shape as a downstream validation
/// failure.
pub(super) fn where_clauses_from_proto(
    clauses: Vec<ProtoWhereClause>,
) -> Result<Vec<WhereClause>, QueryError> {
    shared::where_clauses_from_proto(clauses).map_err(map_decode_error)
}

/// Decode the request-level `repeated OrderClause` field via the
/// shared decoder. Aggregate ordering targets are rejected with
/// `Unsupported("ORDER BY on aggregate keys is not yet implemented")`.
pub(super) fn order_clauses_from_proto(
    clauses: Vec<ProtoOrderClause>,
) -> Result<Vec<OrderClause>, QueryError> {
    shared::order_clauses_from_proto(clauses).map_err(map_decode_error)
}

/// Decode the request-level `repeated HavingClause` field via the
/// shared decoder. Decoding runs before the capability rejection
/// (HAVING evaluation is not implemented) so wire-malformed clauses
/// surface as `InvalidArgument` rather than being masked by the
/// blanket "not yet implemented".
pub(super) fn having_clauses_from_proto(
    clauses: Vec<ProtoHavingClause>,
) -> Result<Vec<HavingClause>, QueryError> {
    shared::having_clauses_from_proto(clauses).map_err(map_decode_error)
}

/// Decode a wire `Select` into drive's [`SelectProjection`] via the
/// shared decoder. Per-function field constraints (e.g. `DOCUMENTS`
/// must have empty `field`, `SUM`/`AVG` require non-empty) are
/// checked at routing time in `validate_and_route`, not here.
pub(super) fn select_from_proto(select: ProtoSelect) -> Result<SelectProjection, QueryError> {
    shared::select_from_proto(select).map_err(map_decode_error)
}
