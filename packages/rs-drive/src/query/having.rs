//! `HAVING` clause types for the v1 `getDocuments` count surface.
//!
//! HAVING differs from WHERE in two structural ways the type
//! system needs to reflect:
//! - The **left** operand is a per-group aggregate (`COUNT(*)`,
//!   `SUM(field)`, `AVG(field)`) rather than a raw row field.
//! - The **right** operand is either a concrete value (`> 5`,
//!   `BETWEEN 5 AND 10`, `IN (5, 10, 15)`) **or** a cross-group
//!   ranking (`EQ MAX`, `IN TOP(5)`, `> MIN`). The ranking
//!   right-operands (`MIN` / `MAX` / `TOP(N)` / `BOTTOM(N)`) are
//!   meta-aggregates computed over the set of group results, so
//!   `HAVING COUNT(*) IN TOP(5)` reads as "this group's count is
//!   among the five largest group counts" — a concise way to
//!   express top-N/bottom-N selection without window functions or
//!   `ORDER BY` + `LIMIT`.
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
//! Only the ranked subset of this grammar executes today — a single
//! `HAVING <agg> IN TOP(n)` / `BOTTOM(n)` clause over a ranked index
//! (see the `drive_document_ranked_query` module).
//! Everything else, value right-operands included, is rejected with
//! `QuerySyntaxError::Unsupported`; the types exist so the wire
//! surface is stable as more of it lands.

use dpp::platform_value::Value;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Aggregate function applied to a group on the left side of a
/// [`HavingClause`]. These are the per-group aggregates whose
/// result is the scalar / numeric value the right-side operand
/// compares against.
///
/// `MIN` / `MAX` / `TOP` / `BOTTOM` deliberately don't appear
/// here — they're cross-group ranking primitives that live on
/// the right side via [`HavingRanking`] (e.g.
/// `HAVING COUNT(*) EQ MAX`).
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

/// Cross-group ranking primitive on the right side of a
/// [`HavingClause`]. The ranking is computed over the **set of
/// group results** (one per row produced by `GROUP BY`), not over
/// the raw rows — so `HAVING COUNT(*) EQ MAX` selects groups
/// whose count equals the maximum count across all groups.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HavingRankingKind {
    /// Smallest group-aggregate value across the result set
    /// (single scalar).
    ///
    /// **Wire-stable but rejected by evaluation.** `= MIN` selects
    /// *every* group tied at the smallest value, and the ranked
    /// storage cannot prove that set: the axis secondary breaks
    /// ties by group key, so a bounded read would silently omit
    /// tied groups and the proof could not attest that nothing
    /// else ties. Use [`Self::Bottom`] with `n = 1` for the
    /// positional "single worst-ranked group", where dropping ties
    /// is the documented meaning.
    Min,
    /// Largest group-aggregate value across the result set
    /// (single scalar).
    ///
    /// **Wire-stable but rejected by evaluation**, symmetrically to
    /// [`Self::Min`]. Use [`Self::Top`] with `n = 1`.
    Max,
    /// Set of the `N` largest group-aggregate values. Pair with
    /// `IN` for membership (`COUNT(*) IN TOP(5)`); single-value
    /// operators (`EQ`, `>`, `<`, …) treat `TOP(1)` as the
    /// maximum.
    Top,
    /// Set of the `N` smallest group-aggregate values. Symmetric
    /// counterpart to [`Self::Top`].
    Bottom,
}

/// Cross-group ranking operand: `kind` plus an optional `n` (only
/// meaningful for [`HavingRankingKind::Top`] /
/// [`HavingRankingKind::Bottom`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HavingRanking {
    /// Which ranking primitive.
    pub kind: HavingRankingKind,
    /// Required for `Top` / `Bottom` (1-indexed: `n=1` is the
    /// single largest / smallest). The wire makes it optional for
    /// forward compatibility, so its absence is rejected at
    /// evaluation rather than at decode. Ignored for `Min` / `Max`,
    /// which evaluation rejects outright whether or not `n` is
    /// present.
    pub n: Option<u64>,
}

/// Right-side operand of a [`HavingClause`]. Either a concrete
/// value (literal scalar or list-shaped operand for
/// `BETWEEN*`/`IN`) or a cross-group ranking reference
/// ([`HavingRanking`]).
///
/// The split lives at the type level so the wire decoder rejects
/// half-built clauses ("operator says `IN`, right side is `MIN`
/// ranking with `n` set") at conversion time rather than letting
/// them reach the evaluator as ambiguous state.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HavingRightOperand {
    /// Concrete value: scalar for `=` / `!=` / `<` / `<=` / `>` /
    /// `>=`; 2-element list `[lower, upper]` for `Between*`;
    /// list of candidates for `In`.
    Value(Value),
    /// Cross-group ranking reference. Operator compatibility:
    /// scalar comparison operators work with `Top(1)` /
    /// `Bottom(1)`; `In` works with `Top(N)` / `Bottom(N)`
    /// (membership in the top-N / bottom-N set). `Min` / `Max` are
    /// wire-stable but rejected by evaluation whatever the
    /// operator — see [`HavingRankingKind::Min`].
    Ranking(HavingRanking),
}

/// Comparison operator for a [`HavingClause`]. Mirrors
/// [`crate::query::WhereOperator`] minus `STARTS_WITH` (prefix
/// matching has no natural meaning against a scalar aggregate
/// result, even a string-typed one). `BETWEEN*` operand semantics
/// match `WhereOperator`: a 2-element list `[lower, upper]`; `IN`
/// expects a list of candidate values (or a cross-group ranking
/// set via [`HavingRightOperand::Ranking`]).
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
    /// Right-side operand — either a concrete value or a
    /// cross-group ranking. See [`HavingRightOperand`] for the
    /// shape contract.
    pub right: HavingRightOperand,
}
