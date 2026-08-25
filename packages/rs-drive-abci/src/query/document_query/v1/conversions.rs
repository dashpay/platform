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
    document_field_value,
    get_documents_request_v1::{select, Select as ProtoSelect},
    having_aggregate, having_clause, order_clause, DocumentFieldValue as ProtoDocumentFieldValue,
    HavingAggregate as ProtoHavingAggregate, HavingClause as ProtoHavingClause,
    OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
    WhereOperator as ProtoWhereOperator,
};
use dpp::platform_value::Value;
use drive::query::TimeRangeGridSpec;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    OrderClause, SelectFunction, SelectProjection, TimeRangeSelector, WhereClause, WhereOperator,
};

/// Map a wire-level [`ProtoWhereOperator`] discriminant onto
/// drive's [`WhereOperator`]. Unknown discriminants are wire-level
/// garbage (no future protocol value would map a malformed integer
/// to a valid behavior), so they surface as
/// [`QueryError::InvalidArgument`] — not `not_yet_implemented`.
pub(super) fn where_operator_from_proto(op: i32) -> Result<WhereOperator, QueryError> {
    let proto_op = ProtoWhereOperator::try_from(op).map_err(|_| {
        QueryError::InvalidArgument(format!(
            "unknown WhereOperator discriminant: {} (valid values: 0..=11, see \
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
        // IN_TIME_RANGE is not an engine operator: it's resolved to a concrete
        // equality from authoritative block time before clause conversion (see
        // `is_time_range_clause` / `time_range_clause_from_proto`), so it must
        // never reach this mapping.
        ProtoWhereOperator::InTimeRange => {
            return Err(QueryError::InvalidArgument(
                "IN_TIME_RANGE where clauses are resolved from block time before \
                 operator conversion and must not be mixed into normal clause decoding"
                    .to_string(),
            ))
        }
    })
}

/// Whether a wire where clause is a time-range selection
/// (`operator == IN_TIME_RANGE`). The v1 handler partitions these out and
/// resolves them from authoritative block time via
/// [`time_range_clause_from_proto`].
pub(super) fn is_time_range_clause(clause: &ProtoWhereClause) -> bool {
    clause.operator == ProtoWhereOperator::InTimeRange as i32
}

/// Decode an `IN_TIME_RANGE` wire where clause into its
/// `(field, selector, grid)`. Two operand shapes:
///
/// - `DocumentFieldValue.text` — the bare selector (`"newest"` /
///   `"oldest"`). Unambiguous only while exactly one grid buckets the
///   field; the resolver rejects it otherwise.
/// - `DocumentFieldValue.list` — `[text(selector), uint64(range),
///   uint64(step)]` or `[…, uint64(phase)]`, naming one grid in the
///   contract's own declared units (seconds). Required when several grids
///   bucket the field. Like the contract grammar and the storage key, a
///   zero phase is spelled by omission — the three-element form — so every
///   grid has exactly one wire spelling.
pub(super) fn time_range_clause_from_proto(
    clause: ProtoWhereClause,
) -> Result<(String, TimeRangeSelector, Option<TimeRangeGridSpec>), QueryError> {
    let field = clause.field;
    let parse_selector = |text: &str| {
        TimeRangeSelector::from_string(text).ok_or_else(|| {
            QueryError::InvalidArgument(format!(
                "IN_TIME_RANGE selector must be \"newest\" or \"oldest\", got \"{}\"",
                text
            ))
        })
    };
    match clause.value.and_then(|v| v.variant) {
        Some(document_field_value::Variant::Text(selector_text)) => {
            Ok((field, parse_selector(&selector_text)?, None))
        }
        Some(document_field_value::Variant::List(list)) => {
            let mut values = list.values.into_iter();
            let selector = match values.next().and_then(|v| v.variant) {
                Some(document_field_value::Variant::Text(s)) => parse_selector(&s)?,
                _ => {
                    return Err(QueryError::InvalidArgument(format!(
                        "IN_TIME_RANGE list operand on field '{}' must start with the \
                         text selector \"newest\" or \"oldest\"",
                        field
                    )))
                }
            };
            let mut grid_number = |name: &str| -> Result<u64, QueryError> {
                match values.next().and_then(|v| v.variant) {
                    Some(document_field_value::Variant::Uint64Value(n)) => Ok(n),
                    _ => Err(QueryError::InvalidArgument(format!(
                        "IN_TIME_RANGE list operand on field '{}' must carry the grid's \
                         {} as a uint64 of seconds, exactly as the contract declares it",
                        field, name
                    ))),
                }
            };
            let range_seconds = grid_number("range")?;
            let step_seconds = grid_number("step")?;
            let phase_seconds = match values.next() {
                None => 0,
                Some(v) => match v.variant {
                    Some(document_field_value::Variant::Uint64Value(0)) => {
                        return Err(QueryError::InvalidArgument(format!(
                            "IN_TIME_RANGE list operand on field '{}': a zero phase is \
                             spelled by omission (use the three-element form), so every \
                             grid has exactly one wire spelling",
                            field
                        )))
                    }
                    Some(document_field_value::Variant::Uint64Value(n)) => n,
                    _ => {
                        return Err(QueryError::InvalidArgument(format!(
                            "IN_TIME_RANGE list operand on field '{}' must carry the \
                             grid's phase as a uint64 of seconds",
                            field
                        )))
                    }
                },
            };
            if values.next().is_some() {
                return Err(QueryError::InvalidArgument(format!(
                    "IN_TIME_RANGE list operand on field '{}' takes at most \
                     [selector, range, step, phase]",
                    field
                )));
            }
            Ok((
                field,
                selector,
                Some(TimeRangeGridSpec {
                    range_seconds,
                    step_seconds,
                    phase_seconds,
                }),
            ))
        }
        _ => Err(QueryError::InvalidArgument(format!(
            "IN_TIME_RANGE clause on field '{}' must carry either a text operand \
             (\"newest\" / \"oldest\") or a list operand [selector, range, step] / \
             [selector, range, step, phase] naming one of the field's grids",
            field
        ))),
    }
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
pub(super) fn value_from_proto(value: ProtoDocumentFieldValue) -> Result<Value, QueryError> {
    value_from_proto_at_depth(value, 0)
}

/// Recursion-bounded form of [`value_from_proto`]. `depth = 0` is
/// the request-level operand; the only legal child shape is a
/// flat list (`depth = 1` for `IN` / `BETWEEN*` candidates), so a
/// `list` encountered at `depth >= 1` is wire-malformed.
fn value_from_proto_at_depth(
    value: ProtoDocumentFieldValue,
    depth: u8,
) -> Result<Value, QueryError> {
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
        document_field_value::Variant::List(list) => {
            if depth >= 1 {
                return Err(QueryError::InvalidArgument(
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
///
/// The `target` oneof currently has two variants on the wire:
/// `field` (plain column name — evaluated today) and `aggregate`
/// (aggregate function applied to a field — wire-only, rejected
/// at routing time with `Unsupported("ORDER BY on aggregate …")`).
/// Unset (`None`) is rejected as malformed wire input.
pub(super) fn order_clause_from_proto(clause: ProtoOrderClause) -> Result<OrderClause, QueryError> {
    let ascending = clause.ascending;
    match clause.target {
        Some(order_clause::Target::Field(field)) => Ok(OrderClause { field, ascending }),
        Some(order_clause::Target::Aggregate(_)) => Err(QueryError::Query(
            drive::error::query::QuerySyntaxError::Unsupported(
                "ORDER BY on aggregate keys is not yet implemented".to_string(),
            ),
        )),
        None => Err(QueryError::InvalidArgument(
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
pub(super) fn order_clauses_from_proto(
    clauses: Vec<ProtoOrderClause>,
) -> Result<Vec<OrderClause>, QueryError> {
    clauses.into_iter().map(order_clause_from_proto).collect()
}

// The `having_*_from_proto` family below decodes clauses the server
// then refuses: `having` evaluation is not implemented, so every
// non-empty HAVING is rejected at routing. Decoding still runs first
// (see `query_documents_v1`) so wire-malformed clauses surface as
// `InvalidArgument` rather than being masked by the capability
// rejection. The inner helpers keep a per-function
// `#[allow(dead_code)]` — rather than module-wide — so any future
// addition outside this family still trips the lint.

/// Map a wire [`having_aggregate::Function`] discriminant onto
/// drive's [`HavingAggregateFunction`]. Unknown discriminants are
/// wire-level garbage (no future protocol value would map a
/// malformed integer to a valid behavior), so they surface as
/// [`QueryError::InvalidArgument`].
#[allow(dead_code)]
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

/// Map a wire [`having_clause::Operator`] discriminant onto
/// drive's [`HavingOperator`]. Same error contract as
/// [`having_function_from_proto`].
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub(super) fn having_clauses_from_proto(
    clauses: Vec<ProtoHavingClause>,
) -> Result<Vec<HavingClause>, QueryError> {
    clauses.into_iter().map(having_clause_from_proto).collect()
}

/// Map a wire [`select::Function`] discriminant onto drive's
/// [`SelectFunction`]. Unknown discriminants are wire-level
/// garbage (no future protocol value would map a malformed
/// integer to a valid behavior), so they surface as
/// [`QueryError::InvalidArgument`].
fn select_function_from_proto(function: i32) -> Result<SelectFunction, QueryError> {
    let proto = select::Function::try_from(function).map_err(|_| {
        QueryError::InvalidArgument(format!(
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
/// routing time in `validate_and_route`, not here, so the
/// converter only enforces well-formed proto.
pub(super) fn select_from_proto(select: ProtoSelect) -> Result<SelectProjection, QueryError> {
    Ok(SelectProjection {
        function: select_function_from_proto(select.function)?,
        field: select.field,
    })
}
