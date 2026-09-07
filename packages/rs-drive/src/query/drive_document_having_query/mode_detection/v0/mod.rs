//! Feature version 0 of the grammar — the frozen implementation
//! behind the `mode_detection` dispatcher. Everything private in
//! this file is a v0 internal: a later grammar version gets its own
//! `vN/` sibling rather than editing this one.

use super::{AxisRangeBounds, DocumentHavingMode, MAX_HAVING_LIMIT};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_ranked_query::mode_detection::{
    prefix_pins_from_where_clauses, ranked_order_key,
};
use crate::query::drive_document_ranked_query::{RankedAxis, RankedPaginationInputs};
use crate::query::having::{
    HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
};
use crate::query::projection::{SelectFunction, SelectProjection};
use crate::query::{OrderClause, WhereClause};
use dpp::platform_value::Value;
use grovedb::element::indexed::AVG_FIXED_POINT_SCALE;

/// v0 of the having-range request grammar.
///
/// Accepts exactly:
///
/// ```text
/// SELECT COUNT(*)  GROUP BY p  HAVING COUNT(*) <op> <value>  [ORDER BY $count [ASC|DESC]]  LIMIT n
/// SELECT SUM(f)    GROUP BY p  HAVING SUM(f)   <op> <value>  [ORDER BY f      [ASC|DESC]]  LIMIT n
/// SELECT AVG(f)    GROUP BY p  HAVING AVG(f)   <op> <value>  [ORDER BY f      [ASC|DESC]]  LIMIT n
/// ```
///
/// with no `OFFSET`, no `START AT` / `START AFTER`, exactly one
/// `GROUP BY` property, `WHERE` clauses (when present) that pin the
/// covering compound ranked index's leading properties — one clause per
/// property, each an equality except that **at most one** clause may be
/// an `IN` whose elements fan the bound out across one prefix branch
/// per element (merged deterministically; see the ranked surface's
/// `prefix_pins_from_where_clauses`) — exactly one `HAVING` clause whose aggregate
/// **is the selected aggregate** (same function, same field), an operator
/// from the contiguous-range family (`=`, `>`, `>=`, `<`, `<=`, and the
/// four `BETWEEN*` variants — `!=` and `IN` describe non-contiguous
/// ranges and are rejected as not yet supported), at most one `ORDER BY`
/// clause naming the selected aggregate, and `1 ≤ n ≤`
/// [`MAX_HAVING_LIMIT`].
///
/// The single-clause / same-aggregate restriction is what makes the
/// query a *range read*: one clause on the selected aggregate is one
/// contiguous slice of one axis secondary. A second clause (implicit
/// AND) or a clause on a different aggregate would need a per-candidate
/// post-check against the primary — a future capability, rejected loudly
/// today.
///
/// Worked examples:
///
/// ```text
/// -- hashtags with more than 100 posts, biggest first
/// SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100 ORDER BY $count DESC LIMIT 100
///
/// -- restaurants averaging a grade of at least 4
/// SELECT AVG(grade) GROUP BY restaurantId HAVING grade >= 4 LIMIT 50
///
/// -- donors whose lifetime total sits between two bounds
/// SELECT SUM(amount) GROUP BY donorId HAVING amount BETWEEN 1000 AND 5000 LIMIT 100
/// ```
///
/// Everything the grammar rejects is rejected *loudly* rather than
/// normalized away — including operator translations that produce an
/// empty range (`> u64::MAX`, `BETWEEN 10 AND 5`): a bound that cannot
/// match any group is a caller error, and silently proving an empty page
/// would hide it.
pub fn detect_having_mode_v0(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    order_by: &[OrderClause],
    where_clauses: &[WhereClause],
    pagination: RankedPaginationInputs,
) -> Result<DocumentHavingMode, Error> {
    // ---- GROUP BY: exactly one property ----------------------------
    //
    // Same contract as the ranked surface: the one property is the
    // covering index's LAST property, whose distinct values are the
    // secondary's group keys. A compound ranked index filters each
    // prefix's groups separately — its leading properties are pinned by
    // `where` clauses (`==`, at most one of them a bounded `IN`), never
    // grouped over.
    if group_by.len() != 1 {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "having-range queries require exactly one `group_by` property (the covering \
             ranked index's trailing property); got {}. A compound ranked index bounds \
             each prefix's groups separately — pin every leading index property with a \
             `where` clause (`==`, or a bounded `IN` on at most one of them) and \
             `group_by` the trailing property.",
            group_by.len()
        ))));
    }
    let group_by_property = group_by[0].clone();
    if group_by_property.is_empty() {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(
            "having-range queries require a non-empty `group_by` property name".to_string(),
        )));
    }

    // ---- SELECT: the axis, and the field it aggregates --------------
    //
    // Same axis resolution as the ranked surface, because the same
    // three secondaries serve both.
    let (axis, aggregate_field) = match (select.function, select.field.as_str()) {
        (SelectFunction::Count, "") => (RankedAxis::Count, String::new()),
        (SelectFunction::Count, field) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "having-range queries support `COUNT(*)` but not `COUNT({field})`; the \
                 count axis counts documents per group, which is what `COUNT(*)` means. \
                 Drop the field to filter by group size."
            ))));
        }
        (SelectFunction::Sum, "") | (SelectFunction::Avg, "") => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                "`SUM` / `AVG` having-range queries require a non-empty select field naming \
                 the index's `summable` property"
                    .to_string(),
            )));
        }
        (SelectFunction::Sum, field) => (RankedAxis::Sum, field.to_string()),
        (SelectFunction::Avg, field) => (RankedAxis::Avg, field.to_string()),
        (other, _) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "having-range queries support `COUNT(*)`, `SUM(field)` and `AVG(field)` \
                 selects; got {other:?}. The bound is served from an indexed tree's \
                 per-axis secondary, and grovedb maintains exactly those three axes."
            ))));
        }
    };

    // ---- HAVING: exactly one clause, on the selected aggregate ------
    let clause = match having {
        [only] => only,
        [] => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                "having-range queries require exactly one `having` clause; got none. \
                 Without a bound the request is a plain grouped aggregate — drop into \
                 that surface instead."
                    .to_string(),
            )));
        }
        many => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "multiple `having` clauses (implicit AND) are not yet supported: got {}. \
                 One clause on the selected aggregate is one contiguous slice of one axis \
                 secondary; a second clause would need a per-candidate post-check against \
                 the primary. Narrow to a single clause.",
                many.len()
            ))));
        }
    };

    let clause_axis = match clause.aggregate.function {
        HavingAggregateFunction::Count => RankedAxis::Count,
        HavingAggregateFunction::Sum => RankedAxis::Sum,
        HavingAggregateFunction::Avg => RankedAxis::Avg,
    };
    if clause_axis != axis || clause.aggregate.field != select.field {
        return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "the `having` clause must bound the selected aggregate itself: the select is \
             `{:?}({})` but the clause bounds `{:?}({})`. Filtering by one aggregate while \
             projecting another would need a per-candidate post-check against the primary, \
             which is not yet supported.",
            select.function,
            if select.field.is_empty() {
                "*"
            } else {
                select.field.as_str()
            },
            clause.aggregate.function,
            if clause.aggregate.field.is_empty() {
                "*"
            } else {
                clause.aggregate.field.as_str()
            }
        ))));
    }

    // ---- Operator + right operand → inclusive bounds -----------------
    let HavingRightOperand::Value(right) = &clause.right;
    let bounds = match axis {
        RankedAxis::Count => {
            let (lo, hi) = bounds_for_operator(
                clause.operator,
                right,
                count_operand,
                u64::MIN,
                u64::MAX,
                |v| v.checked_add(1),
                |v| v.checked_sub(1),
            )?;
            AxisRangeBounds::Count { lo, hi }
        }
        RankedAxis::Sum => {
            let (lo, hi) = bounds_for_operator(
                clause.operator,
                right,
                sum_operand,
                i64::MIN,
                i64::MAX,
                |v| v.checked_add(1),
                |v| v.checked_sub(1),
            )?;
            AxisRangeBounds::Sum { lo, hi }
        }
        RankedAxis::Avg => {
            let (lo, hi) = avg_bounds_for_operator(clause.operator, right)?;
            AxisRangeBounds::Avg { lo, hi }
        }
    };

    // ---- ORDER BY: absent (ascending default) or the aggregate ------
    //
    // Optional here, unlike ranked, because the bound — not the
    // ordering — is what the query is about. When present it must name
    // the selected aggregate: matching groups come off a single-key
    // secondary, so there is no other order the walk could serve.
    let expected_order_key = ranked_order_key(select);
    let descending = match order_by {
        [] => false,
        [only] if only.field == expected_order_key => !only.ascending,
        [only] => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "`HAVING … ORDER BY {}` is not supported: matching groups are read off the \
                 axis secondary, so the only ordering available is the bounded aggregate \
                 itself — write `ORDER BY {expected_order_key}` or omit `order_by` for \
                 ascending.",
                only.field
            ))));
        }
        many => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "having-range queries accept at most one `order_by` clause (naming the \
                 selected aggregate); got {}. The axis secondary is a single-key ordering, \
                 so there is no second sort key to apply.",
                many.len()
            ))));
        }
    };

    // ---- WHERE: pins on the compound prefix ---------------------------
    //
    // Identical contract to the ranked surface: empty for the
    // single-property form; for a compound ranked index, one pin per
    // leading property — `==`, except at most one bounded branching
    // `IN` — selects which prefix secondary or secondaries the bound
    // reads. Shape-only here; the index picker enforces the exact-cover
    // rule.
    let prefix_pins = prefix_pins_from_where_clauses(where_clauses)?;

    // ---- LIMIT: required, 1 ..= MAX_HAVING_LIMIT ---------------------
    //
    // Required rather than defaulted for the same reason as ranked: the
    // limit is part of the traversal the verifier re-executes the proof
    // against, so there is no server default a client could reproduce.
    // Required *especially* here, because a threshold can match
    // unboundedly many groups.
    let limit = pagination.limit.ok_or_else(|| {
        Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "having-range queries require an explicit `limit` (1 ..= {MAX_HAVING_LIMIT}): \
             a bound can match any number of groups, the walk stops at `limit`, and the \
             verifier re-executes the proof under the limit it rebuilds from the request, \
             so there is no server-side default a client could reproduce."
        )))
    })?;
    if limit == 0 {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "`LIMIT 0` selects nothing; having-range queries require 1 ≤ limit ≤ {MAX_HAVING_LIMIT}"
        ))));
    }
    if limit > MAX_HAVING_LIMIT as u32 {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "`LIMIT {limit}` exceeds the having-range ceiling of {MAX_HAVING_LIMIT}; the \
             proof commits one secondary entry per returned group, so its size grows \
             linearly in the limit. The ceiling is a hard limit, not a clamp, because \
             the limit is part of the traversal the client rebuilds to verify — a clamped \
             walk is one the client's reconstruction did not ask for. Narrow the bound to \
             shrink the result set."
        ))));
    }
    // Bounded by MAX_HAVING_LIMIT (a u16) immediately above.
    let limit = limit as u16;

    // ---- OFFSET / START AT: must be absent ---------------------------
    //
    // The range primitives take a limit but no skip, and unlike the
    // ranked walk there is no counted-commitment shortcut for "skip m
    // matching groups" — pagination of an over-long match set is a
    // future cursor capability on the `(sort_key ‖ group_key)`
    // keyspace, not an emulated offset. Rejected loudly, `OFFSET 0`
    // included: a caller writing any offset asked for pagination
    // semantics this surface does not have.
    if pagination.offset.is_some() {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(
            "having-range queries do not accept `offset`: matching groups are read from \
             the bound's start and cut at `limit`. To reach deeper matches, tighten the \
             bound past the last aggregate value already seen — noting that a page cut \
             inside a tie (several groups sharing the boundary aggregate) cannot be \
             continued that way; size `limit` above the widest expected tie."
                .to_string(),
        )));
    }
    if pagination.has_start_at {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(
            "having-range queries do not accept `start_at` / `start_after`: the cursor \
             names a document id, which does not appear in a keyspace sorted by \
             aggregate."
                .to_string(),
        )));
    }

    Ok(DocumentHavingMode {
        bounds,
        descending,
        limit,
        group_by_property,
        aggregate_field,
        prefix_pins,
    })
}

/// Translate `(operator, right operand)` into inclusive `[lo, hi]`
/// bounds in one axis's value domain.
///
/// `operand` extracts a single scalar from a [`Value`] in that domain;
/// `succ` / `pred` are the domain's checked successor / predecessor,
/// used to normalize the exclusive operators (`>`, `<`, the `BETWEEN`
/// exclusions) onto inclusive bounds. A `succ`/`pred` that overflows
/// means the operator excludes the entire domain past its own extreme
/// (`> MAX`, `< MIN`) — rejected, like every other empty translation,
/// rather than served as a proof of nothing.
fn bounds_for_operator<T: Copy + PartialOrd + std::fmt::Display>(
    operator: HavingOperator,
    right: &Value,
    operand: impl Fn(&Value) -> Result<T, Error>,
    min: T,
    max: T,
    succ: impl Fn(T) -> Option<T>,
    pred: impl Fn(T) -> Option<T>,
) -> Result<(T, T), Error> {
    let scalar = || operand(right);
    let pair = || -> Result<(T, T), Error> {
        let Some(items) = right.as_array() else {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "`{operator:?}` requires a 2-element list operand `[lower, upper]`; got a \
                 non-list value"
            ))));
        };
        let [lower, upper] = items.as_slice() else {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "`{operator:?}` requires a 2-element list operand `[lower, upper]`; got {} \
                 element(s)",
                items.len()
            ))));
        };
        Ok((operand(lower)?, operand(upper)?))
    };
    let strictly_above = |v: T| {
        succ(v).ok_or_else(|| {
            Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "`{operator:?} {v}` matches no possible aggregate value: {v} is the \
                 largest value the aggregate can take"
            )))
        })
    };
    let strictly_below = |v: T| {
        pred(v).ok_or_else(|| {
            Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "`{operator:?} {v}` matches no possible aggregate value: {v} is the \
                 smallest value the aggregate can take"
            )))
        })
    };

    let (lo, hi) = match operator {
        HavingOperator::Equal => {
            let v = scalar()?;
            (v, v)
        }
        HavingOperator::GreaterThan => (strictly_above(scalar()?)?, max),
        HavingOperator::GreaterThanOrEquals => (scalar()?, max),
        HavingOperator::LessThan => (min, strictly_below(scalar()?)?),
        HavingOperator::LessThanOrEquals => (min, scalar()?),
        HavingOperator::Between => pair()?,
        HavingOperator::BetweenExcludeBounds => {
            let (lower, upper) = pair()?;
            (strictly_above(lower)?, strictly_below(upper)?)
        }
        HavingOperator::BetweenExcludeLeft => {
            let (lower, upper) = pair()?;
            (strictly_above(lower)?, upper)
        }
        HavingOperator::BetweenExcludeRight => {
            let (lower, upper) = pair()?;
            (lower, strictly_below(upper)?)
        }
        HavingOperator::NotEqual | HavingOperator::In => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "`{operator:?}` is not yet supported in having-range queries: it describes \
                 a non-contiguous set of aggregate values, and the axis secondary serves \
                 one contiguous range per request. Use a range operator, or issue one \
                 request per contiguous range."
            ))));
        }
    };

    if lo > hi {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "the `having` bound resolves to the empty range [{lo}, {hi}] (lower above \
             upper), which matches no group; fix the operand"
        ))));
    }
    Ok((lo, hi))
}

/// Extract a count operand: a non-negative integer.
fn count_operand(value: &Value) -> Result<u64, Error> {
    value.to_integer::<u64>().map_err(|_| {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "a `COUNT(*)` having bound must be a non-negative integer; got {value}"
        )))
    })
}

/// Extract a sum operand: a signed integer in `i64` range.
fn sum_operand(value: &Value) -> Result<i64, Error> {
    value.to_integer::<i64>().map_err(|_| {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "a `SUM(field)` having bound must be an integer within i64 range; got {value}"
        )))
    })
}

/// An `AVG` operand scaled into the axis's fixed-point domain, kept as
/// `(⌊t × SCALE⌋, is the product exactly that integer?)` so the operator
/// translation can pick the correct floor or ceiling per bound.
///
/// A plain truncated `i128` would be wrong for floats: truncation is
/// toward zero, but an inclusive lower bound needs the *ceiling* and an
/// upper bound the *floor* — and around zero the two diverge in
/// opposite directions (`AVG >= 0.5-tick` must start at tick 1, while
/// truncation says 0; `AVG > -0.5-tick` must start at tick 0, while
/// truncate-then-increment says 1).
#[derive(Debug, Clone, Copy)]
struct ScaledAvgOperand {
    /// `⌊t × SCALE⌋` — the floor (toward −∞) of the exact real product.
    floor: i128,
    /// Whether `t × SCALE` is exactly `floor` (the operand lands on a
    /// fixed-point tick). Always true for integer operands.
    exact: bool,
}

/// Extract an average operand and scale it into the axis's fixed-point
/// domain (see
/// [`super::super::drive_document_ranked_query::RANKED_AVG_SCALE`])
/// **exactly**.
///
/// Integer operands scale exactly (`v × SCALE` — the product of any i64
/// with the scale fits in `i128` by the compile-time bound next to the
/// scale constant). Float operands are decomposed into their IEEE-754
/// `±mantissa × 2^exponent` form and the product `±mantissa × SCALE ×
/// 2^exponent` is floored with integer arithmetic — never through an
/// `f64` multiplication, which loses sub-tick precision long before
/// this scale (`SCALE = 10^19 > 2^53`): the nearest-f64 rounding of
/// `t × SCALE` can land on the wrong side of a tick and silently move
/// an inclusive bound by one.
fn scaled_avg_operand(value: &Value) -> Result<ScaledAvgOperand, Error> {
    if let Some(int) = value.as_integer::<i64>() {
        let floor = (int as i128)
            .checked_mul(AVG_FIXED_POINT_SCALE)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidParameter(format!(
                    "the `AVG(field)` having bound {int} does not fit the fixed-point domain"
                )))
            })?;
        return Ok(ScaledAvgOperand { floor, exact: true });
    }
    let float = value.to_float().map_err(|_| {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "an `AVG(field)` having bound must be an integer or a float; got {value}"
        )))
    })?;
    if !float.is_finite() {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "an `AVG(field)` having bound must be finite; got {float}"
        ))));
    }

    // IEEE-754 double decomposition: float = ±mantissa × 2^exponent,
    // with the implicit leading bit restored for normal numbers and the
    // subnormal exponent pinned at 2^-1074.
    let bits = float.to_bits();
    let negative = bits >> 63 == 1;
    let raw_exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let (mantissa, exponent) = if raw_exponent == 0 {
        (fraction, -1074i64)
    } else {
        (fraction | 0x0010_0000_0000_0000, raw_exponent - 1075)
    };
    if mantissa == 0 {
        // ±0.0 — exactly tick zero.
        return Ok(ScaledAvgOperand {
            floor: 0,
            exact: true,
        });
    }
    let out_of_domain = || {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "the `AVG(field)` having bound {float} does not fit the fixed-point domain"
        )))
    };

    // mantissa < 2^54 and SCALE < 2^64, so the product stays well under
    // i128::MAX (< 2^118); only the 2^exponent factor can overflow.
    let magnitude = (mantissa as i128) * AVG_FIXED_POINT_SCALE;
    let signed = if negative { -magnitude } else { magnitude };
    if exponent >= 0 {
        // × 2^exponent, exactly. |signed| ≥ SCALE ≥ 1, so a factor the
        // domain cannot hold means the bound itself is out of domain.
        if exponent >= 127 {
            return Err(out_of_domain());
        }
        let floor = signed
            .checked_mul(1i128 << exponent)
            .ok_or_else(out_of_domain)?;
        Ok(ScaledAvgOperand { floor, exact: true })
    } else {
        // ÷ 2^-exponent with euclidean (toward −∞) division — exactly
        // the floor, with the remainder deciding exactness.
        let shift = -exponent as u32;
        if shift >= 127 {
            // |signed| < 2^118 < 2^shift ⇒ 0 < |t × SCALE| < 1: the
            // product floors to 0 (positive) or −1 (negative), and is
            // never exact (mantissa is non-zero).
            return Ok(ScaledAvgOperand {
                floor: if negative { -1 } else { 0 },
                exact: false,
            });
        }
        let divisor = 1i128 << shift;
        Ok(ScaledAvgOperand {
            floor: signed.div_euclid(divisor),
            exact: signed.rem_euclid(divisor) == 0,
        })
    }
}

/// Translate `(operator, right operand)` into inclusive `[lo, hi]`
/// bounds in the Avg axis's fixed-point domain.
///
/// The Avg counterpart of [`bounds_for_operator`], separate because Avg
/// operands may be floats that do not land on a fixed-point tick, and
/// the correct translation is then **operator-aware**: an inclusive
/// lower bound takes the ceiling of the exact product `t × SCALE`, an
/// upper bound its floor, and the exclusive translations collapse onto
/// the inclusive ones whenever `t` sits strictly between two ticks
/// (`v > t` and `v ≥ t` admit exactly the same integers there). All of
/// it works off [`scaled_avg_operand`]'s exact `(floor, exact)` pair:
///
/// | operator      | lower bound            | upper bound            |
/// |---------------|------------------------|------------------------|
/// | `= t`         | `t` exact on a tick — otherwise rejected: nothing can match |
/// | `> t`         | `⌊t⌋ + 1`              | domain max             |
/// | `>= t`        | `⌈t⌉`                  | domain max             |
/// | `< t`         | domain min             | `⌈t⌉ − 1`              |
/// | `<= t`        | domain min             | `⌊t⌋`                  |
/// | `BETWEEN*`    | per-end combination of the four rows above      |
///
/// Empty translations (`> MAX`, a between pair that inverts, an
/// equality between ticks) are rejected loudly, matching
/// [`bounds_for_operator`]'s contract: a bound that cannot match any
/// group is a caller error, and silently proving an empty page would
/// hide it.
fn avg_bounds_for_operator(operator: HavingOperator, right: &Value) -> Result<(i128, i128), Error> {
    let scalar = || scaled_avg_operand(right);
    let pair = || -> Result<(ScaledAvgOperand, ScaledAvgOperand), Error> {
        let Some(items) = right.as_array() else {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "`{operator:?}` requires a 2-element list operand `[lower, upper]`; got a \
                 non-list value"
            ))));
        };
        let [lower, upper] = items.as_slice() else {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "`{operator:?}` requires a 2-element list operand `[lower, upper]`; got {} \
                 element(s)",
                items.len()
            ))));
        };
        Ok((scaled_avg_operand(lower)?, scaled_avg_operand(upper)?))
    };
    let past_max = || {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "the `{operator:?}` bound matches no possible aggregate value: it lies at or \
             above the largest value the aggregate can take"
        )))
    };
    let past_min = || {
        Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "the `{operator:?}` bound matches no possible aggregate value: it lies at or \
             below the smallest value the aggregate can take"
        )))
    };

    // v > t ⇔ v ≥ ⌊t⌋ + 1 whether or not t is a tick (for a tick,
    // strictly above it; between ticks, the ceiling).
    let exclusive_lower = |bound: ScaledAvgOperand| bound.floor.checked_add(1).ok_or_else(past_max);
    // v ≥ t ⇔ v ≥ ⌈t⌉.
    let inclusive_lower = |bound: ScaledAvgOperand| {
        if bound.exact {
            Ok(bound.floor)
        } else {
            bound.floor.checked_add(1).ok_or_else(past_max)
        }
    };
    // v < t ⇔ v ≤ ⌈t⌉ − 1 (t on a tick: strictly below it; between
    // ticks: the floor).
    let exclusive_upper = |bound: ScaledAvgOperand| {
        if bound.exact {
            bound.floor.checked_sub(1).ok_or_else(past_min)
        } else {
            Ok(bound.floor)
        }
    };
    // v ≤ t ⇔ v ≤ ⌊t⌋, which never overflows.
    let inclusive_upper = |bound: ScaledAvgOperand| bound.floor;

    let (lo, hi) = match operator {
        HavingOperator::Equal => {
            let bound = scalar()?;
            if !bound.exact {
                return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                    "the `AVG(field)` equality bound {right} does not land on a fixed-point \
                     tick, so no group's average can equal it; use a range operator (e.g. \
                     `BETWEEN`) around the intended value, or an operand that scales exactly"
                ))));
            }
            (bound.floor, bound.floor)
        }
        HavingOperator::GreaterThan => (exclusive_lower(scalar()?)?, i128::MAX),
        HavingOperator::GreaterThanOrEquals => (inclusive_lower(scalar()?)?, i128::MAX),
        HavingOperator::LessThan => (i128::MIN, exclusive_upper(scalar()?)?),
        HavingOperator::LessThanOrEquals => (i128::MIN, inclusive_upper(scalar()?)),
        HavingOperator::Between => {
            let (lower, upper) = pair()?;
            (inclusive_lower(lower)?, inclusive_upper(upper))
        }
        HavingOperator::BetweenExcludeBounds => {
            let (lower, upper) = pair()?;
            (exclusive_lower(lower)?, exclusive_upper(upper)?)
        }
        HavingOperator::BetweenExcludeLeft => {
            let (lower, upper) = pair()?;
            (exclusive_lower(lower)?, inclusive_upper(upper))
        }
        HavingOperator::BetweenExcludeRight => {
            let (lower, upper) = pair()?;
            (inclusive_lower(lower)?, exclusive_upper(upper)?)
        }
        HavingOperator::NotEqual | HavingOperator::In => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "`{operator:?}` is not yet supported in having-range queries: it describes \
                 a non-contiguous set of aggregate values, and the axis secondary serves \
                 one contiguous range per request. Use a range operator, or issue one \
                 request per contiguous range."
            ))));
        }
    };

    if lo > hi {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "the `having` bound resolves to the empty range [{lo}, {hi}] (lower above \
             upper), which matches no group; fix the operand"
        ))));
    }
    Ok((lo, hi))
}
