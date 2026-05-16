//! Wire-protobuf → drive type conversions for the v1 document
//! query surface.
//!
//! Lives next to the v1 handler because rs-drive-abci is the only
//! crate that needs the proto-decode direction (the SDK ships the
//! inverse direction in
//! `rs-sdk/src/platform/documents/document_query.rs`). Keeping the
//! two directions in their respective crates avoids forcing
//! `dapi-grpc` into rs-drive's dependency graph just to host shared
//! conversion code.
//!
//! Conversion contract:
//! - Every fallible case maps to [`QueryError::InvalidArgument`]
//!   (malformed wire input, **not** future capability). The v1
//!   handler distinguishes this from
//!   [`QuerySyntaxError::Unsupported`] (valid request shape, server
//!   capability not yet wired) — see `v1/mod.rs`'s
//!   `not_yet_implemented` helper.
//! - Conversion is schema-agnostic. `DocumentFieldValue` variants
//!   map 1:1 to `dpp::platform_value::Value` variants without
//!   consulting the document type's schema. The schema-driven
//!   coercion (`document_type.serialize_value_for_key`) runs
//!   downstream as it does for the CBOR-shaped v0 path — a `text`
//!   variant against an identifier field decodes via base58, a
//!   `bytes_value` against the same field decodes as raw 32-byte
//!   identifier, and so on. The wire layer just names the
//!   primitive; the schema decides the indexed type.

use crate::error::query::QueryError;
use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value, DocumentFieldValue as ProtoDocumentFieldValue,
    OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
    WhereOperator as ProtoWhereOperator,
};
use dpp::platform_value::Value;
use drive::query::{OrderClause, WhereClause, WhereOperator};

/// Map a wire-level [`ProtoWhereOperator`] discriminant onto
/// drive's [`WhereOperator`]. Unknown discriminants are wire-level
/// garbage (no future protocol value would map a malformed integer
/// to a valid behavior), so they surface as
/// [`QueryError::InvalidArgument`] — not `not_yet_implemented`.
pub(super) fn where_operator_from_proto(op: i32) -> Result<WhereOperator, QueryError> {
    let proto_op = ProtoWhereOperator::try_from(op).map_err(|_| {
        QueryError::InvalidArgument(format!(
            "unknown WhereOperator discriminant: {} (valid values: 0..=10, see \
             `get_documents_request::WhereOperator`)",
            op
        ))
    })?;
    Ok(match proto_op {
        ProtoWhereOperator::Equal => WhereOperator::Equal,
        ProtoWhereOperator::GreaterThan => WhereOperator::GreaterThan,
        ProtoWhereOperator::GreaterThanOrEquals => WhereOperator::GreaterThanOrEquals,
        ProtoWhereOperator::LessThan => WhereOperator::LessThan,
        ProtoWhereOperator::LessThanOrEquals => WhereOperator::LessThanOrEquals,
        ProtoWhereOperator::Between => WhereOperator::Between,
        ProtoWhereOperator::BetweenExcludeBounds => WhereOperator::BetweenExcludeBounds,
        ProtoWhereOperator::BetweenExcludeLeft => WhereOperator::BetweenExcludeLeft,
        ProtoWhereOperator::BetweenExcludeRight => WhereOperator::BetweenExcludeRight,
        ProtoWhereOperator::In => WhereOperator::In,
        ProtoWhereOperator::StartsWith => WhereOperator::StartsWith,
    })
}

/// Map a wire [`ProtoDocumentFieldValue`] onto a
/// `dpp::platform_value::Value`. Schema-agnostic — variants map
/// 1:1 by primitive type and recurse for `list`.
///
/// `None` (oneof unset on the wire) is rejected — a where-clause
/// operand is always concrete; empty where-clauses are expressed
/// by an empty `where_clauses` field at the request level, not by
/// sending an empty `DocumentFieldValue`.
pub(super) fn value_from_proto(value: ProtoDocumentFieldValue) -> Result<Value, QueryError> {
    let variant = value.variant.ok_or_else(|| {
        QueryError::InvalidArgument(
            "DocumentFieldValue has no variant set; a where-clause operand must \
             be a concrete value"
                .to_string(),
        )
    })?;
    Ok(match variant {
        document_field_value::Variant::BoolValue(b) => Value::Bool(b),
        document_field_value::Variant::Int64Value(i) => Value::I64(i),
        document_field_value::Variant::Uint64Value(u) => Value::U64(u),
        document_field_value::Variant::DoubleValue(f) => Value::Float(f),
        document_field_value::Variant::Text(s) => Value::Text(s),
        document_field_value::Variant::BytesValue(b) => Value::Bytes(b),
        document_field_value::Variant::List(list) => Value::Array(
            list.values
                .into_iter()
                .map(value_from_proto)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    })
}

/// Map a wire [`ProtoWhereClause`] onto drive's structured
/// [`WhereClause`]. Errors surface as
/// [`QueryError::InvalidArgument`] for both operator-discriminant
/// and value-shape failures.
pub(super) fn where_clause_from_proto(clause: ProtoWhereClause) -> Result<WhereClause, QueryError> {
    let operator = where_operator_from_proto(clause.operator)?;
    let value = clause.value.ok_or_else(|| {
        QueryError::InvalidArgument(format!(
            "WhereClause on field '{}' has no value set; every clause must carry a \
             concrete `DocumentFieldValue`",
            clause.field
        ))
    })?;
    let value = value_from_proto(value)?;
    Ok(WhereClause {
        field: clause.field,
        operator,
        value,
    })
}

/// Plural form of [`where_clause_from_proto`] for the request-level
/// `repeated WhereClause` field. Returns an error on the first
/// malformed clause; the v1 handler surfaces this through
/// `QueryValidationResult::new_with_error` so the caller sees the
/// rejection on the same response shape as a downstream validation
/// failure.
pub(super) fn where_clauses_from_proto(
    clauses: Vec<ProtoWhereClause>,
) -> Result<Vec<WhereClause>, QueryError> {
    clauses.into_iter().map(where_clause_from_proto).collect()
}

/// Map a wire [`ProtoOrderClause`] onto drive's [`OrderClause`].
/// 1:1 field copy — both sides carry the same `(field, ascending)`
/// pair.
pub(super) fn order_clause_from_proto(clause: ProtoOrderClause) -> OrderClause {
    OrderClause {
        field: clause.field,
        ascending: clause.ascending,
    }
}

/// Plural form of [`order_clause_from_proto`] for the request-level
/// `repeated OrderClause` field.
pub(super) fn order_clauses_from_proto(clauses: Vec<ProtoOrderClause>) -> Vec<OrderClause> {
    clauses.into_iter().map(order_clause_from_proto).collect()
}
