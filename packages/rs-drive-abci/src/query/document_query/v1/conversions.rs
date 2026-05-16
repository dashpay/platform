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
    document_field_value, having_aggregate, having_clause, having_ranking,
    DocumentFieldValue as ProtoDocumentFieldValue, HavingAggregate as ProtoHavingAggregate,
    HavingClause as ProtoHavingClause, HavingRanking as ProtoHavingRanking,
    OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
    WhereOperator as ProtoWhereOperator,
};
use dpp::platform_value::Value;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRanking,
    HavingRankingKind, HavingRightOperand, OrderClause, WhereClause, WhereOperator,
};

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
        // The bool payload is a placeholder — picking the
        // `null_value` variant means "this operand is null" and
        // the bool itself is ignored. See the proto-side comment
        // on the field for the rationale.
        document_field_value::Variant::NullValue(_) => Value::Null,
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

/// Map a wire [`having_aggregate::Function`] discriminant onto
/// drive's [`HavingAggregateFunction`]. Unknown discriminants are
/// wire-level garbage (no future protocol value would map a
/// malformed integer to a valid behavior), so they surface as
/// [`QueryError::InvalidArgument`].
fn having_function_from_proto(function: i32) -> Result<HavingAggregateFunction, QueryError> {
    let proto = having_aggregate::Function::try_from(function).map_err(|_| {
        QueryError::InvalidArgument(format!(
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

/// Map a wire [`having_ranking::Kind`] discriminant onto drive's
/// [`HavingRankingKind`].
fn having_ranking_kind_from_proto(kind: i32) -> Result<HavingRankingKind, QueryError> {
    let proto = having_ranking::Kind::try_from(kind).map_err(|_| {
        QueryError::InvalidArgument(format!(
            "unknown HavingRanking.Kind discriminant: {} (valid values: 0..=3, see \
             `get_documents_request::having_ranking::Kind`)",
            kind
        ))
    })?;
    Ok(match proto {
        having_ranking::Kind::Min => HavingRankingKind::Min,
        having_ranking::Kind::Max => HavingRankingKind::Max,
        having_ranking::Kind::Top => HavingRankingKind::Top,
        having_ranking::Kind::Bottom => HavingRankingKind::Bottom,
    })
}

/// Map a wire [`ProtoHavingRanking`] onto drive's [`HavingRanking`].
/// The `kind` ↔ `n` consistency check (e.g. `n` required for
/// `Top` / `Bottom`, forbidden on `Min` / `Max`) runs inside the
/// evaluator when HAVING execution lands; this converter only
/// enforces that the proto shape is well-formed.
fn having_ranking_from_proto(ranking: ProtoHavingRanking) -> Result<HavingRanking, QueryError> {
    Ok(HavingRanking {
        kind: having_ranking_kind_from_proto(ranking.kind)?,
        n: ranking.n,
    })
}

/// Map a wire [`having_clause::Operator`] discriminant onto
/// drive's [`HavingOperator`]. Same error contract as
/// [`having_function_from_proto`].
fn having_operator_from_proto(operator: i32) -> Result<HavingOperator, QueryError> {
    let proto = having_clause::Operator::try_from(operator).map_err(|_| {
        QueryError::InvalidArgument(format!(
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
) -> Result<HavingAggregate, QueryError> {
    Ok(HavingAggregate {
        function: having_function_from_proto(aggregate.function)?,
        field: aggregate.field,
    })
}

/// Map a wire [`ProtoHavingClause`] onto drive's structured
/// [`HavingClause`]. Errors surface as
/// [`QueryError::InvalidArgument`] for any wire-level
/// malformation: unknown discriminant on the aggregate function,
/// operator, or ranking kind; missing aggregate; missing right
/// operand (oneof unset on the wire); inner value-shape failures
/// on the literal-value branch.
pub(super) fn having_clause_from_proto(
    clause: ProtoHavingClause,
) -> Result<HavingClause, QueryError> {
    let aggregate = clause.aggregate.ok_or_else(|| {
        QueryError::InvalidArgument(
            "HavingClause has no aggregate set; every clause must carry an \
             aggregate function + field operand"
                .to_string(),
        )
    })?;
    let aggregate = having_aggregate_from_proto(aggregate)?;
    let operator = having_operator_from_proto(clause.operator)?;
    let right = clause.right.ok_or_else(|| {
        QueryError::InvalidArgument(
            "HavingClause has no right operand set; every clause must carry \
             either a concrete `DocumentFieldValue` (`right.value`) or a \
             cross-group ranking reference (`right.ranking`)"
                .to_string(),
        )
    })?;
    let right = match right {
        having_clause::Right::Value(v) => HavingRightOperand::Value(value_from_proto(v)?),
        having_clause::Right::Ranking(r) => {
            HavingRightOperand::Ranking(having_ranking_from_proto(r)?)
        }
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
pub(super) fn having_clauses_from_proto(
    clauses: Vec<ProtoHavingClause>,
) -> Result<Vec<HavingClause>, QueryError> {
    clauses.into_iter().map(having_clause_from_proto).collect()
}
