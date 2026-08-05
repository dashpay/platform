//! `HAVING` clause types for the v1 `getDocuments` aggregate surface.
//!
//! `HAVING` is a **boolean predicate evaluated per group**, exactly as
//! in SQL. It differs from `WHERE` in one structural way the type
//! system needs to reflect: the left operand is a per-group aggregate
//! (`COUNT(*)`, `SUM(field)`, `AVG(field)`) rather than a raw row
//! field. The right operand is always a concrete value (`> 5`,
//! `BETWEEN 5 AND 10`, `IN (5, 10, 15)`).
//!
//! **Ranking does not live here.** "Which groups score highest on the
//! selected aggregate" is spelled with SQL's own ordering surface —
//! `ORDER BY <selected aggregate> [ASC|DESC] LIMIT n OFFSET m` — and is
//! resolved by
//! [`crate::query::drive_document_ranked_query::mode_detection`]. An
//! earlier iteration of this module carried `TOP(n)` / `BOTTOM(n)` /
//! `MIN` / `MAX` right-operands; they were removed because they
//! duplicated `ORDER BY … LIMIT` with a second, non-SQL grammar that
//! could not express an offset.
//!
//! The operator set matches [`crate::query::WhereOperator`] minus
//! `STARTS_WITH` (prefix matching has no meaning on a scalar
//! aggregate result, even one that's a string): scalar comparison,
//! `IN`, and all four `BETWEEN*` variants all carry through.
//!
//! Multi-clause HAVING (`HAVING COUNT(*) > 5 AND SUM(amount) > 100`)
//! is expressed by repeating [`HavingClause`] at the request
//! level — implicit AND, same shape as multiple `where_clauses`
//! entries.
//!
//! These types are shared between the wire-decoding layer
//! (`rs-drive-abci/src/query/document_query/v1/conversions.rs`)
//! and the SDK's request builder
//! (`rs-sdk/src/platform/documents/document_query.rs`) so the
//! drive-side struct is the single source of truth for the shape.
//! **No part of this grammar executes today**: every non-empty
//! `having` is rejected with `QuerySyntaxError::Unsupported`. The
//! types exist so the wire surface is stable as evaluation lands.

use dpp::platform_value::Value;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Aggregate function applied to a group on the left side of a
/// [`HavingClause`]. These are the per-group aggregates whose
/// result is the scalar / numeric value the right-side operand
/// compares against.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HavingAggregateFunction {
    /// `COUNT(*)` when [`HavingAggregate::field`] is empty,
    /// otherwise `COUNT(field)`.
    Count,
    /// `SUM(field)`. Numeric field required.
    Sum,
    /// `AVG(field)`. Numeric field required; result is `f64`.
    Avg,
}

/// Aggregate operand for the left side of a [`HavingClause`]. See
/// [`HavingAggregateFunction`] for the per-function `field`
/// requirements (empty only for `COUNT(*)`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingAggregate {
    /// The aggregate function applied to the group.
    pub function: HavingAggregateFunction,
    /// The field the aggregate is applied to. Empty only when
    /// `function == Count` (to express `COUNT(*)`).
    pub field: String,
}

/// Right-side operand of a [`HavingClause`]: a concrete value
/// (literal scalar, or list-shaped operand for `BETWEEN*` / `IN`).
///
/// Kept as a single-variant enum rather than collapsed into a bare
/// `Value` field on [`HavingClause`] because the wire models the
/// right operand as a `oneof`: an enum is what a `oneof` decodes
/// into, and a future right-operand kind (a correlated subquery, a
/// reference to another aggregate) lands as a variant here instead
/// of reshaping every consumer's field access.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HavingRightOperand {
    /// Concrete value: scalar for `=` / `!=` / `<` / `<=` / `>` /
    /// `>=`; 2-element list `[lower, upper]` for `Between*`;
    /// list of candidates for `In`.
    Value(Value),
}

/// Comparison operator for a [`HavingClause`]. Mirrors
/// [`crate::query::WhereOperator`] minus `STARTS_WITH` (prefix
/// matching has no natural meaning against a scalar aggregate
/// result, even a string-typed one). `BETWEEN*` operand semantics
/// match `WhereOperator`: a 2-element list `[lower, upper]`; `IN`
/// expects a list of candidate values.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HavingOperator {
    /// `aggregate = value`.
    Equal,
    /// `aggregate != value`.
    NotEqual,
    /// `aggregate > value`.
    GreaterThan,
    /// `aggregate >= value`.
    GreaterThanOrEquals,
    /// `aggregate < value`.
    LessThan,
    /// `aggregate <= value`.
    LessThanOrEquals,
    /// `aggregate BETWEEN lower AND upper` (inclusive on both
    /// ends). `value` must be a 2-element list `[lower, upper]`.
    Between,
    /// `aggregate > lower AND aggregate < upper` (exclusive on
    /// both ends). `value` shape same as `Between`.
    BetweenExcludeBounds,
    /// `aggregate > lower AND aggregate <= upper` (exclusive on
    /// the left bound only). `value` shape same as `Between`.
    BetweenExcludeLeft,
    /// `aggregate >= lower AND aggregate < upper` (exclusive on
    /// the right bound only). `value` shape same as `Between`.
    BetweenExcludeRight,
    /// `aggregate IN (v1, v2, …)`. `value` must be a list of
    /// candidate values matching the aggregate's result type.
    In,
}

/// Single `HAVING <aggregate> <op> <right>` clause.
///
/// Multiple [`HavingClause`] entries in the request-level
/// `repeated HavingClause having` field are combined with implicit
/// `AND` — same semantics as multiple `where_clauses` entries.
/// `HAVING COUNT(*) > 5 AND SUM(amount) > 100` is two clauses, not
/// a tree; the wire has no dedicated `AND` node because the
/// repeated field already expresses it. Future `OR` capability
/// would land as an additional wire shape (e.g. a `HavingGroup`
/// message with a logical-op tag) rather than overloading this
/// type.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingClause {
    /// Left-side per-group aggregate operand.
    pub aggregate: HavingAggregate,
    /// Comparison operator.
    pub operator: HavingOperator,
    /// Right-side operand. See [`HavingRightOperand`] for the
    /// shape contract.
    pub right: HavingRightOperand,
}
