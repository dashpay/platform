//! `HAVING` clause types for the v1 `getDocuments` count surface.
//!
//! HAVING differs from WHERE in two ways the type system needs to
//! reflect:
//! - The left-hand operand is an **aggregate** over the group
//!   (`COUNT(*)`, `SUM(field)`, `AVG(field)`, `MIN`/`MAX`, `TOP`/
//!   `BOTTOM` for N-th-element selection), not a raw row field.
//! - The operator set is narrower than `WhereOperator`. Aggregate
//!   results are scalars (or N-th-element selections), so `IN` /
//!   `BETWEEN` / `STARTS_WITH` have no natural meaning here —
//!   HAVING only supports the six scalar comparisons.
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
//! The server currently rejects any non-empty `having` with
//! `QuerySyntaxError::Unsupported("HAVING clause is not yet
//! implemented")` — the types exist so the wire surface is stable
//! when execution lands.

use dpp::platform_value::Value;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Aggregate function applied to a group on the left side of a
/// [`HavingClause`].
///
/// **Field semantics by function**:
/// - [`Self::Count`]: empty `field` on the enclosing
///   [`HavingAggregate`] means `COUNT(*)` (group cardinality);
///   non-empty `field` means `COUNT(field)` (count of non-null
///   values of `field` in the group).
/// - [`Self::Sum`] / [`Self::Avg`] / [`Self::Min`] / [`Self::Max`]:
///   `field` is required. Numeric-typed fields only — the server
///   rejects with a typed error on string / bytes targets when
///   evaluation lands.
/// - [`Self::Top`] / [`Self::Bottom`]: N-th-element aggregates.
///   `Top(field)` against right-operand `N` evaluates to "the N-th
///   largest value of `field` in the group"; `Bottom(field)` is
///   the symmetric N-th-smallest. The `N` argument lives in the
///   `HavingClause.value` slot, not here, so a single
///   `HavingAggregate` is enough to express both `field` and
///   `function`. Useful for "at least N members above threshold"
///   without an explicit fan-out via `ORDER BY` + `LIMIT`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HavingAggregateFunction {
    /// `COUNT(*)` when `HavingAggregate.field` is empty, otherwise
    /// `COUNT(field)`.
    Count,
    /// `SUM(field)`. Numeric field required.
    Sum,
    /// `AVG(field)`. Numeric field required; result is `f64`.
    Avg,
    /// `MIN(field)`. Comparable field required.
    Min,
    /// `MAX(field)`. Comparable field required.
    Max,
    /// `TOP(field)` — N-th-largest value of `field` in the group;
    /// `N` comes from `HavingClause.value`.
    Top,
    /// `BOTTOM(field)` — N-th-smallest value of `field` in the
    /// group; `N` comes from `HavingClause.value`.
    Bottom,
}

/// Aggregate operand `(<function>, <field>)` for the left side of
/// a [`HavingClause`]. See [`HavingAggregateFunction`] for the
/// per-function `field` semantics (in particular: `field` is empty
/// only for `COUNT(*)`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingAggregate {
    /// The aggregate function applied to the group.
    pub function: HavingAggregateFunction,
    /// The field the aggregate is applied to. Empty only when
    /// `function == Count` (to express `COUNT(*)`).
    pub field: String,
}

/// Comparison operator for a [`HavingClause`]. Narrower than
/// `WhereOperator` because HAVING evaluates against scalar
/// aggregate results — `IN` / `BETWEEN` / `STARTS_WITH` have no
/// natural meaning against a single aggregate value, and would
/// require redefining the operand shape to a list / range tuple
/// without any current semantic backing.
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
}

/// Single `HAVING <aggregate> <op> <value>` clause.
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
///
/// The `value` slot holds the right-hand operand. For
/// [`HavingAggregateFunction::Top`] / [`HavingAggregateFunction::Bottom`]
/// this is the `N` argument (a positive integer); for all other
/// functions it's the comparison target.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingClause {
    /// Left-side aggregate operand.
    pub aggregate: HavingAggregate,
    /// Comparison operator.
    pub operator: HavingOperator,
    /// Right-side scalar value. For `TOP` / `BOTTOM` aggregates,
    /// this is the `N` argument (positive integer); for other
    /// aggregates it's the comparison target whose type must match
    /// the aggregate's result type (numeric for `SUM`/`AVG`/`COUNT`,
    /// the field's indexed type for `MIN`/`MAX`).
    pub value: Value,
}
