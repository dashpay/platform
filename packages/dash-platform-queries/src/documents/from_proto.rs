//! Wire → drive decoding of document where clauses, for state transition subscriptions. Drive
//! ABCI's `query/document_query/v1/conversions.rs` keeps its own copy; the two must stay in
//! sync. Decoding is schema-agnostic, and every error describes malformed wire input.

use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value, DocumentFieldValue as ProtoDocumentFieldValue,
    WhereClause as ProtoWhereClause, WhereOperator as ProtoWhereOperator,
};
use dpp::platform_value::Value;
use drive::query::{WhereClause, WhereOperator};

/// Map a wire [`ProtoWhereOperator`] discriminant onto drive's [`WhereOperator`]. The window
/// selections (`IN_TIME_RANGE`, `IN_INTEGER_RANGE`) are not engine operators and are refused.
pub fn where_operator_from_proto(op: i32) -> Result<WhereOperator, String> {
    let proto_op = ProtoWhereOperator::try_from(op).map_err(|_| {
        format!(
            "unknown WhereOperator discriminant: {} (valid values: 0..=14, see \
             `get_documents_request::WhereOperator`)",
            op
        )
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
        // The elements of a typed array: Drive's grammar, index admissibility
        // and lowering judge them (protocol version 14).
        ProtoWhereOperator::Contains => WhereOperator::Contains,
        ProtoWhereOperator::ContainsAny => WhereOperator::ContainsAny,
        // IN_TIME_RANGE is resolved to a concrete equality from authoritative
        // block time before clause conversion, so it must never reach here.
        ProtoWhereOperator::InTimeRange => {
            return Err(
                "IN_TIME_RANGE where clauses are resolved from block time before \
                 operator conversion and must not be mixed into normal clause decoding"
                    .to_string(),
            )
        }
        // IN_INTEGER_RANGE is likewise resolved to a window-start equality
        // before clause conversion.
        ProtoWhereOperator::InIntegerRange => {
            return Err(
                "IN_INTEGER_RANGE where clauses are resolved to a window-start equality \
                 before operator conversion and must not be mixed into normal clause decoding"
                    .to_string(),
            )
        }
    })
}

/// Map a wire [`ProtoDocumentFieldValue`] onto a `Value`. A `list` is accepted one level deep
/// only (the `IN` / `BETWEEN*` candidates), so a hostile client cannot nest lists to exhaust
/// the stack before schema validation.
pub fn value_from_proto(value: ProtoDocumentFieldValue) -> Result<Value, String> {
    value_from_proto_at_depth(value, 0)
}

fn value_from_proto_at_depth(value: ProtoDocumentFieldValue, depth: u8) -> Result<Value, String> {
    let variant = value.variant.ok_or_else(|| {
        "DocumentFieldValue has no variant set; a where-clause operand must \
         be a concrete value"
            .to_string()
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
                return Err("nested DocumentFieldValue.list is not supported; the v1 \
                     query surface accepts at most one level of nesting \
                     (`IN` / `BETWEEN*` candidate lists of scalars)"
                    .to_string());
            }
            Value::Array(
                list.values
                    .into_iter()
                    .map(|v| value_from_proto_at_depth(v, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        // Only the variant carries meaning; its bool payload is a placeholder.
        document_field_value::Variant::NullValue(_) => Value::Null,
    })
}

/// Map a wire [`ProtoWhereClause`] onto drive's [`WhereClause`]. A clause carrying a window
/// selection operand (`time_range`, `integer_range`) is refused.
pub fn where_clause_from_proto(clause: ProtoWhereClause) -> Result<WhereClause, String> {
    let operator = where_operator_from_proto(clause.operator)?;
    if clause.time_range.is_some() {
        return Err(format!(
            "WhereClause on field '{}' sets `time_range`, which is only valid with the \
             IN_TIME_RANGE operator",
            clause.field
        ));
    }
    if clause.integer_range.is_some() {
        return Err(format!(
            "WhereClause on field '{}' sets `integer_range`, which is only valid with the \
             IN_INTEGER_RANGE operator",
            clause.field
        ));
    }
    let value = clause.value.ok_or_else(|| {
        format!(
            "WhereClause on field '{}' has no value set; every clause must carry a \
             concrete `DocumentFieldValue`",
            clause.field
        )
    })?;
    Ok(WhereClause {
        field: clause.field,
        operator,
        value: value_from_proto(value)?,
    })
}

/// [`where_clause_from_proto`] for every clause, failing on the first malformed one.
pub fn where_clauses_from_proto(
    clauses: Vec<ProtoWhereClause>,
) -> Result<Vec<WhereClause>, String> {
    clauses.into_iter().map(where_clause_from_proto).collect()
}
