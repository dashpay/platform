//! Wire-protobuf → drive type conversions for the `getDocuments`
//! query surface.
//!
//! This is the **single** proto-decode implementation, shared by:
//! - rs-drive-abci's v1 request handler (server side — decodes the
//!   incoming request before routing/execution), and
//! - [`DocumentQuery::try_from_request`](super::document_query::DocumentQuery::try_from_request)
//!   (client side — rebuilds the rich query from the wire request so
//!   a proved response can be verified against exactly what was
//!   asked).
//!
//! Both directions living on one implementation is the point: the
//! bytes the server decodes and the bytes the verifier decodes must
//! agree clause-for-clause, or a proof could verify against a
//! different query than the server answered.
//!
//! Conversion contract:
//! - Every fallible case maps to [`DecodeError::InvalidArgument`]
//!   (malformed wire input, **not** future capability), except the
//!   aggregate `ORDER BY` target which maps to
//!   [`DecodeError::Unsupported`] (valid request shape, server
//!   capability not yet wired). rs-drive-abci maps these onto its
//!   `QueryError::InvalidArgument` / `QuerySyntaxError::Unsupported`
//!   respectively, preserving its historical error surface.
//! - Conversion is schema-agnostic. `DocumentFieldValue` variants
//!   map 1:1 to `dpp::platform_value::Value` variants without
//!   consulting the document type's schema. The schema-driven
//!   coercion (`document_type.serialize_value_for_key`) runs
//!   downstream as it does for the CBOR-shaped v0 path — a `text`
//!   variant against an identifier field decodes via base58, a
//!   `bytes_value` against the same field decodes as raw 32-byte
//!   identifier, and so on. The wire layer just names the
//!   primitive; the schema decides the indexed type.

use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value,
    get_documents_request_v1::{select, Select as ProtoSelect},
    having_aggregate, having_clause, order_clause, DocumentFieldValue as ProtoDocumentFieldValue,
    HavingAggregate as ProtoHavingAggregate, HavingClause as ProtoHavingClause,
    OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
    WhereOperator as ProtoWhereOperator,
};
use dpp::platform_value::Value;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    OrderClause, SelectFunction, SelectProjection, WhereClause, WhereOperator,
};

/// Neutral decode error for the shared proto → drive conversions.
///
/// Deliberately not a server or client error type: rs-drive-abci
/// maps it onto its `QueryError`, and the client-side
/// `DocumentQuery` decoding maps it onto the crate
/// [`Error`](crate::error::Error), each preserving its own error
/// surface.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// Malformed wire input — bad discriminant, missing oneof arm,
    /// over-deep list nesting. No future protocol version would make
    /// this input valid.
    #[error("{0}")]
    InvalidArgument(String),
    /// Well-formed wire input naming a capability the decode target
    /// cannot represent yet (e.g. `ORDER BY` on an aggregate key).
    /// The wording signals future capability, not malformed request.
    #[error("{0}")]
    Unsupported(String),
}

/// Map a wire-level [`ProtoWhereOperator`] discriminant onto
/// drive's [`WhereOperator`]. Unknown discriminants are wire-level
/// garbage (no future protocol value would map a malformed integer
/// to a valid behavior), so they surface as
/// [`DecodeError::InvalidArgument`].
pub fn where_operator_from_proto(op: i32) -> Result<WhereOperator, DecodeError> {
    let proto_op = ProtoWhereOperator::try_from(op).map_err(|_| {
        DecodeError::InvalidArgument(format!(
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
/// 1:1 by primitive type and recurse for `list` up to a depth of
/// 1 (the only nesting level the query surface needs: `IN` /
/// `BETWEEN*` take a flat list of scalars). Anything deeper is
/// rejected as malformed wire input rather than recursed into,
/// so a hostile client can't blow the call stack with
/// `list(list(list(...)))` before schema validation.
///
/// `None` (oneof unset on the wire) is rejected — a where-clause
/// operand is always concrete; empty where-clauses are expressed
/// by an empty `where_clauses` field at the request level, not by
/// sending an empty `DocumentFieldValue`.
pub fn value_from_proto(value: ProtoDocumentFieldValue) -> Result<Value, DecodeError> {
    value_from_proto_at_depth(value, 0)
}

/// Recursion-bounded form of [`value_from_proto`]. `depth = 0` is
/// the request-level operand; the only legal child shape is a
/// flat list (`depth = 1` for `IN` / `BETWEEN*` candidates), so a
/// `list` encountered at `depth >= 1` is wire-malformed.
fn value_from_proto_at_depth(
    value: ProtoDocumentFieldValue,
    depth: u8,
) -> Result<Value, DecodeError> {
    let variant = value.variant.ok_or_else(|| {
        DecodeError::InvalidArgument(
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
        document_field_value::Variant::List(list) => {
            if depth >= 1 {
                return Err(DecodeError::InvalidArgument(
                    "nested DocumentFieldValue.list is not supported; the v1 \
                     query surface accepts at most one level of nesting \
                     (`IN` / `BETWEEN*` candidate lists of scalars)"
                        .to_string(),
                ));
            }
            Value::Array(
                list.values
                    .into_iter()
                    .map(|v| value_from_proto_at_depth(v, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        // The bool payload is a placeholder — picking the
        // `null_value` variant means "this operand is null" and
        // the bool itself is ignored. See the proto-side comment
        // on the field for the rationale.
        document_field_value::Variant::NullValue(_) => Value::Null,
    })
}

/// Map a wire [`ProtoWhereClause`] onto drive's structured
/// [`WhereClause`]. Errors surface as
/// [`DecodeError::InvalidArgument`] for both operator-discriminant
/// and value-shape failures.
pub fn where_clause_from_proto(clause: ProtoWhereClause) -> Result<WhereClause, DecodeError> {
    let operator = where_operator_from_proto(clause.operator)?;
    let value = clause.value.ok_or_else(|| {
        DecodeError::InvalidArgument(format!(
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
/// malformed clause.
pub fn where_clauses_from_proto(
    clauses: Vec<ProtoWhereClause>,
) -> Result<Vec<WhereClause>, DecodeError> {
    clauses.into_iter().map(where_clause_from_proto).collect()
}

/// Map a wire [`ProtoOrderClause`] onto drive's [`OrderClause`].
///
/// The `target` oneof currently has two variants on the wire:
/// `field` (plain column name — evaluated today) and `aggregate`
/// (aggregate function applied to a field — wire-only, rejected
/// with [`DecodeError::Unsupported`]). Unset (`None`) is rejected
/// as malformed wire input.
pub fn order_clause_from_proto(clause: ProtoOrderClause) -> Result<OrderClause, DecodeError> {
    let ascending = clause.ascending;
    match clause.target {
        Some(order_clause::Target::Field(field)) => Ok(OrderClause { field, ascending }),
        Some(order_clause::Target::Aggregate(_)) => Err(DecodeError::Unsupported(
            "ORDER BY on aggregate keys is not yet implemented".to_string(),
        )),
        None => Err(DecodeError::InvalidArgument(
            "OrderClause has no target set; every clause must carry either a \
             `field` (plain column name) or an `aggregate` (aggregate-function \
             ordering target)"
                .to_string(),
        )),
    }
}

/// Plural form of [`order_clause_from_proto`] for the request-level
/// `repeated OrderClause` field. Returns the first error
/// encountered.
pub fn order_clauses_from_proto(
    clauses: Vec<ProtoOrderClause>,
) -> Result<Vec<OrderClause>, DecodeError> {
    clauses.into_iter().map(order_clause_from_proto).collect()
}

/// Map a wire [`having_aggregate::Function`] discriminant onto
/// drive's [`HavingAggregateFunction`]. Unknown discriminants are
/// wire-level garbage (no future protocol value would map a
/// malformed integer to a valid behavior), so they surface as
/// [`DecodeError::InvalidArgument`].
fn having_function_from_proto(function: i32) -> Result<HavingAggregateFunction, DecodeError> {
    let proto = having_aggregate::Function::try_from(function).map_err(|_| {
        DecodeError::InvalidArgument(format!(
            "unknown HavingAggregate.Function discriminant: {} (valid values: 0..=2, see \
             `get_documents_request::having_aggregate::Function`)",
            function
        ))
    })?;
    Ok(match proto {
        having_aggregate::Function::Count => HavingAggregateFunction::Count,
        having_aggregate::Function::Sum => HavingAggregateFunction::Sum,
        having_aggregate::Function::Avg => HavingAggregateFunction::Avg,
    })
}

/// Map a wire [`having_clause::Operator`] discriminant onto
/// drive's [`HavingOperator`]. Same error contract as
/// [`having_function_from_proto`].
fn having_operator_from_proto(operator: i32) -> Result<HavingOperator, DecodeError> {
    let proto = having_clause::Operator::try_from(operator).map_err(|_| {
        DecodeError::InvalidArgument(format!(
            "unknown HavingClause.Operator discriminant: {} (valid values: 0..=10, see \
             `get_documents_request::having_clause::Operator`)",
            operator
        ))
    })?;
    Ok(match proto {
        having_clause::Operator::Equal => HavingOperator::Equal,
        having_clause::Operator::NotEqual => HavingOperator::NotEqual,
        having_clause::Operator::GreaterThan => HavingOperator::GreaterThan,
        having_clause::Operator::GreaterThanOrEquals => HavingOperator::GreaterThanOrEquals,
        having_clause::Operator::LessThan => HavingOperator::LessThan,
        having_clause::Operator::LessThanOrEquals => HavingOperator::LessThanOrEquals,
        having_clause::Operator::Between => HavingOperator::Between,
        having_clause::Operator::BetweenExcludeBounds => HavingOperator::BetweenExcludeBounds,
        having_clause::Operator::BetweenExcludeLeft => HavingOperator::BetweenExcludeLeft,
        having_clause::Operator::BetweenExcludeRight => HavingOperator::BetweenExcludeRight,
        having_clause::Operator::In => HavingOperator::In,
    })
}

/// Map a wire [`ProtoHavingAggregate`] onto drive's
/// [`HavingAggregate`]. The aggregate-function ↔ field
/// consistency check (`field` required for everything except
/// `Count`) runs inside the evaluator when HAVING execution
/// lands; the converter only enforces that the proto shape is
/// well-formed.
fn having_aggregate_from_proto(
    aggregate: ProtoHavingAggregate,
) -> Result<HavingAggregate, DecodeError> {
    Ok(HavingAggregate {
        function: having_function_from_proto(aggregate.function)?,
        field: aggregate.field,
    })
}

/// Map a wire [`ProtoHavingClause`] onto drive's structured
/// [`HavingClause`]. Errors surface as
/// [`DecodeError::InvalidArgument`] for any wire-level
/// malformation: unknown discriminant on the aggregate function or
/// operator; missing aggregate; missing right operand (oneof unset
/// on the wire); inner value-shape failures on the literal-value
/// branch.
///
/// `HAVING` is a boolean per-group predicate and nothing else, so the
/// wire's `right` oneof has exactly one arm and this function has
/// exactly one thing to decode. Cross-group ranking is expressed with
/// SQL's own ordering surface — `ORDER BY <the selected aggregate> DESC
/// LIMIT n [OFFSET m]` — which arrives as an `OrderClause` and never
/// reaches here.
pub fn having_clause_from_proto(clause: ProtoHavingClause) -> Result<HavingClause, DecodeError> {
    let aggregate = clause.aggregate.ok_or_else(|| {
        DecodeError::InvalidArgument(
            "HavingClause has no aggregate set; every clause must carry an \
             aggregate function + field operand"
                .to_string(),
        )
    })?;
    let aggregate = having_aggregate_from_proto(aggregate)?;
    let operator = having_operator_from_proto(clause.operator)?;
    let right = clause.right.ok_or_else(|| {
        DecodeError::InvalidArgument(
            "HavingClause has no right operand set; every clause must carry a \
             concrete `DocumentFieldValue` (`right.value`)"
                .to_string(),
        )
    })?;
    let right = match right {
        having_clause::Right::Value(v) => HavingRightOperand::Value(value_from_proto(v)?),
    };
    Ok(HavingClause {
        aggregate,
        operator,
        right,
    })
}

/// Plural form of [`having_clause_from_proto`] for the request-
/// level `repeated HavingClause` field. Returns an error on the
/// first malformed clause.
pub fn having_clauses_from_proto(
    clauses: Vec<ProtoHavingClause>,
) -> Result<Vec<HavingClause>, DecodeError> {
    clauses.into_iter().map(having_clause_from_proto).collect()
}

/// Map a wire [`select::Function`] discriminant onto drive's
/// [`SelectFunction`]. Unknown discriminants are wire-level
/// garbage (no future protocol value would map a malformed
/// integer to a valid behavior), so they surface as
/// [`DecodeError::InvalidArgument`].
fn select_function_from_proto(function: i32) -> Result<SelectFunction, DecodeError> {
    let proto = select::Function::try_from(function).map_err(|_| {
        DecodeError::InvalidArgument(format!(
            "unknown Select.Function discriminant: {} (valid values: 0..=5, see \
             `get_documents_request::get_documents_request_v1::select::Function`)",
            function
        ))
    })?;
    Ok(match proto {
        select::Function::Documents => SelectFunction::Documents,
        select::Function::Count => SelectFunction::Count,
        select::Function::Sum => SelectFunction::Sum,
        select::Function::Avg => SelectFunction::Avg,
        select::Function::Min => SelectFunction::Min,
        select::Function::Max => SelectFunction::Max,
    })
}

/// Map a wire [`ProtoSelect`] onto drive's [`SelectProjection`].
/// An unset `select` field on the request decodes as the proto-
/// default `Select { function: DOCUMENTS, field: "" }`, which
/// maps to [`SelectProjection::documents()`] — keeps callers that
/// don't set the field on the v0-style document-fetch path.
///
/// Per-function field constraints (e.g. `DOCUMENTS` must have
/// empty `field`, `SUM`/`AVG` require non-empty) are checked at
/// routing time by the server's `validate_and_route`, not here, so
/// the converter only enforces well-formed proto.
pub fn select_from_proto(select: ProtoSelect) -> Result<SelectProjection, DecodeError> {
    Ok(SelectProjection {
        function: select_function_from_proto(select.function)?,
        field: select.field,
    })
}
