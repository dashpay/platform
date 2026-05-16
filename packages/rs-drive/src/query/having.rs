//! `HAVING` clause types for the v1 `getDocuments` count surface.
//!
//! HAVING differs from WHERE in one structural way the type system
//! needs to reflect: the left-hand operand is an **aggregate** over
//! the group (`COUNT(*)`, `SUM(field)`, `AVG(field)`, `MIN`/`MAX`,
//! `TOP`/`BOTTOM` for N-th-element selection) rather than a raw
//! row field. The operator set is the same as `WhereOperator`
//! minus `STARTS_WITH` (prefix matching has no meaning on a
//! scalar aggregate result, even one that's a string): scalar
//! comparison, `IN`, and all four `BETWEEN*` variants all carry
//! through.
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
/// **Field semantics by function** (refer to
/// [`HavingAggregate::field`]):
/// - [`Self::Count`]: empty `field` means `COUNT(*)` (group
///   cardinality); non-empty `field` means `COUNT(field)` (count
///   of non-null values of `field` in the group).
/// - [`Self::Sum`] / [`Self::Avg`] / [`Self::Min`] / [`Self::Max`]:
///   `field` is required. Numeric-typed fields only for `Sum` /
///   `Avg`; comparable types for `Min` / `Max`. The server rejects
///   with a typed error on incompatible field types when
///   evaluation lands.
/// - [`Self::Top`] / [`Self::Bottom`]: N-th-element aggregates.
///   `Top(field)` evaluates to "the N-th largest value of `field`
///   in the group", and `Bottom(field)` is the symmetric
///   N-th-smallest. The `N` argument lives in
///   [`HavingAggregate::n`] (1-indexed); the `HavingClause.value`
///   slot stays free for the comparison target so all operators
///   (`=`, range, `IN`, `BETWEEN*`) work uniformly with these
///   functions.
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
    /// `TOP(field, N)` — N-th-largest value of `field` in the
    /// group. `N` lives in [`HavingAggregate::n`] (1-indexed).
    Top,
    /// `BOTTOM(field, N)` — N-th-smallest value of `field` in the
    /// group. `N` lives in [`HavingAggregate::n`] (1-indexed).
    Bottom,
}

/// Aggregate operand for the left side of a [`HavingClause`]. See
/// [`HavingAggregateFunction`] for the per-function `field` /
/// `n` requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingAggregate {
    /// The aggregate function applied to the group.
    pub function: HavingAggregateFunction,
    /// The field the aggregate is applied to. Empty only when
    /// `function == Count` (to express `COUNT(*)`).
    pub field: String,
    /// N-th rank for [`HavingAggregateFunction::Top`] /
    /// [`HavingAggregateFunction::Bottom`] (1-indexed: `n=1` is
    /// the largest / smallest element). Required for those two
    /// functions; must be `None` for the others (the wire still
    /// allows it for forward-compatibility, but evaluation
    /// rejects it as a malformed aggregate).
    pub n: Option<u64>,
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
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingClause {
    /// Left-side aggregate operand. For `TOP` / `BOTTOM` the
    /// N-th-rank argument lives on the aggregate's `n` field, not
    /// here — `value` is reserved for the comparison target.
    pub aggregate: HavingAggregate,
    /// Comparison operator.
    pub operator: HavingOperator,
    /// Right-side operand. Shape depends on `operator`:
    /// scalar comparison operators expect a scalar value whose
    /// type matches the aggregate's result type (numeric for
    /// `SUM`/`AVG`/`COUNT`, the field's type for `MIN` / `MAX` /
    /// `TOP` / `BOTTOM`); `Between*` expects a 2-element list
    /// `[lower, upper]`; `In` expects a list of candidate values.
    pub value: Value,
}
