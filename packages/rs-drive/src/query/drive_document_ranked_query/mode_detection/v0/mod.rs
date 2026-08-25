//! Feature version 0 of the grammar — the frozen implementation
//! behind the `mode_detection` dispatcher. Everything private in
//! this file is a v0 internal: a later grammar version gets its own
//! `vN/` sibling rather than editing this one.

use super::super::{PrefixPin, MAX_PREFIX_IN_BRANCHES};
use super::ranked_order_key;
use super::{
    DocumentRankedMode, RankedAxis, RankedPaginationInputs, MAX_RANKED_LIMIT,
    RANKED_COUNT_ORDER_KEY,
};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::having::HavingClause;
use crate::query::projection::{SelectFunction, SelectProjection};
use crate::query::{OrderClause, WhereClause, WhereOperator};
use dpp::platform_value::Value;

/// Translate a request's `where` clauses into prefix pins — one
/// [`PrefixPin`] per clause, carrying the pinned property and its
/// value(s): one value from an `==` clause, several from the (at most
/// one) `IN` — for the ranked and having-range surfaces.
///
/// Both surfaces read a compound index's per-prefix secondary by
/// descending through one prefix value tree per **leading** index
/// property, and only an equality clause names a single value tree to
/// descend into. So the grammar is: every `where` clause is an equality
/// (`==`) on a distinct property, except that **at most one** clause may
/// be an `IN` — each of its elements selects its own prefix *branch*,
/// and the executors walk one secondary per branch and merge (see
/// [`MAX_PREFIX_IN_BRANCHES`] for the fan-out ceiling). A
/// range operator on a prefix property can never pin a subtree and is
/// rejected outright. A single-element `IN` is normalized to an
/// equality pin, so the degenerate case is byte-identical to `==`.
///
/// Shape-only, like everything in this module: whether the pinned
/// properties are exactly the leading properties of a covering compound
/// index is the index picker's call, and element distinctness is
/// enforced post-encoding by the prefix encoder (two spellings of one
/// value are one branch, and must be rejected as a duplicate).
pub fn prefix_pins_from_where_clauses(
    where_clauses: &[WhereClause],
) -> Result<Vec<PrefixPin>, Error> {
    let mut pins: Vec<PrefixPin> = Vec::with_capacity(where_clauses.len());
    let mut branching_field: Option<&str> = None;
    for clause in where_clauses {
        if clause.field.is_empty() {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "a ranked / having-range query's `where` clause names an empty property",
                ),
            ));
        }
        if pins.iter().any(|pin| pin.field == clause.field) {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "a ranked / having-range query pins the same property twice: each leading \
                 index property takes exactly one pin",
                ),
            ));
        }
        let values = match clause.operator {
            WhereOperator::Equal => vec![clause.value.clone()],
            WhereOperator::In => {
                let Value::Array(elements) = &clause.value else {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "an `IN` pin's right operand must be an array of candidate values",
                        ),
                    ));
                };
                if elements.is_empty() {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "an `IN` pin's element list is empty: it can match nothing, and \
                             a pin that cannot match any prefix is a caller error",
                        ),
                    ));
                }
                if elements.len() > MAX_PREFIX_IN_BRANCHES {
                    return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                        "an `IN` pin fans out into one secondary walk (and one proof \
                         branch) per element; {} elements exceeds the ceiling of {}. \
                         Split the request.",
                        elements.len(),
                        MAX_PREFIX_IN_BRANCHES
                    ))));
                }
                // Only a multi-element `IN` branches; a singleton is an
                // equality pin and never counts against the one-`IN`
                // budget — in either clause order.
                if elements.len() > 1 {
                    if let Some(first) = branching_field {
                        return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                            "a ranked / having-range query takes at most one branching \
                             `IN` across its prefix properties (`{first}` already carries \
                             it): several `IN`s multiply into a cartesian product of \
                             prefix branches, each a separate secondary walk inside one \
                             proof — a fan-out the branch ceiling exists to prevent. Pin \
                             `{}` with `==` or a single-element `IN`.",
                            clause.field
                        ))));
                    }
                    branching_field = Some(clause.field.as_str());
                }
                elements.clone()
            }
            _ => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "a ranked / having-range query's `where` clauses must pin the covering \
                     compound index's leading properties with `==` (or one `IN`): the \
                     per-prefix secondary lives under one prefix value tree per leading \
                     property, and only equality names value trees to descend into — a \
                     range operator cannot pin a prefix",
                    ),
                ));
            }
        };
        pins.push(PrefixPin {
            field: clause.field.clone(),
            values,
        });
    }
    Ok(pins)
}

/// v0 of the ranked request grammar.
///
/// Accepts exactly:
///
/// ```text
/// SELECT COUNT(*)  [WHERE q1 = v1 [AND …]]  GROUP BY p  ORDER BY $count [ASC|DESC]  LIMIT n [OFFSET m]
/// SELECT SUM(f)    [WHERE q1 = v1 [AND …]]  GROUP BY p  ORDER BY f      [ASC|DESC]  LIMIT n [OFFSET m]
/// SELECT AVG(f)    [WHERE q1 = v1 [AND …]]  GROUP BY p  ORDER BY f      [ASC|DESC]  LIMIT n [OFFSET m]
/// ```
///
/// with no `HAVING`, no `START AT` / `START AFTER`, exactly one
/// `GROUP BY` property, exactly one `ORDER BY` clause naming the
/// selected aggregate, `1 ≤ n ≤` [`MAX_RANKED_LIMIT`], and any
/// `m ≥ 0`. `WHERE` clauses, when present, pin distinct properties —
/// one per leading property of a covering compound ranked index — each
/// an **equality**, except that at most one may be a bounded **`IN`**
/// (2..=10 distinct elements; a singleton `IN` normalizes to `==` — see
/// [`prefix_pins_from_where_clauses`]). A `==`-pinned request reads
/// that pinned prefix's own secondary; an `IN` fans the read out into
/// one prefix branch per element, walked separately and merged
/// deterministically, with merged entries carrying their branch's
/// `in_key`. A **non-zero** `OFFSET` is rejected together with the
/// `IN`: the counted rank-skip is attested per-secondary and cannot
/// span the union (`OFFSET 0` stays legal as the offset-free
/// spelling). With no `where` the covering index is single-property,
/// exactly as before.
///
/// `DESC` walks the axis from the largest aggregate down (the "top n"
/// reading), `ASC` from the smallest up (the "bottom n" reading).
///
/// Worked examples:
///
/// ```text
/// -- the three best restaurants by average grade
/// SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT 3
///
/// -- the single worst restaurant by average grade
/// SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade ASC LIMIT 1
///
/// -- the *5th* best grade: skip the four above it, take one
/// SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT 1 OFFSET 4
///
/// -- the busiest ten restaurants, second page
/// SELECT COUNT(*) GROUP BY restaurantId ORDER BY $count DESC LIMIT 10 OFFSET 10
/// ```
///
/// Everything the grammar rejects is rejected *loudly* rather than
/// normalized away: a caller who wrote a filter, a `having`, or an
/// ordering on a property the select does not aggregate asked for
/// something this executor cannot deliver, and silently answering a
/// different question is worse than an error.
pub fn detect_ranked_mode_v0(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    order_by: &[OrderClause],
    where_clauses: &[WhereClause],
    pagination: RankedPaginationInputs,
) -> Result<DocumentRankedMode, Error> {
    // ---- GROUP BY: exactly one property ----------------------------
    //
    // The one property is the covering ranked index's LAST property —
    // the level whose distinct values are the secondary's group keys.
    // Zero group_by would ask to rank a single global aggregate against
    // itself. Two or more is rejected because a compound ranked index
    // ranks per prefix, not across a compound grouping: its leading
    // properties are pinned by equality `where` clauses, and only the
    // trailing property is grouped over.
    if group_by.len() != 1 {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
            "ranked queries require exactly one `group_by` property (the covering ranked \
             index's trailing property); got {}. A compound ranked index ranks each \
             prefix's groups separately — pin every leading index property with an \
             equality `where` clause and `group_by` the trailing property.",
            group_by.len()
        ))));
    }
    let group_by_property = group_by[0].clone();
    if group_by_property.is_empty() {
        return Err(Error::Query(QuerySyntaxError::InvalidParameter(
            "ranked queries require a non-empty `group_by` property name".to_string(),
        )));
    }

    // ---- SELECT: the axis, and the field it aggregates --------------
    let (axis, aggregate_field) = match (select.function, select.field.as_str()) {
        (SelectFunction::Count, "") => (RankedAxis::Count, String::new()),
        (SelectFunction::Count, field) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "ranked queries support `COUNT(*)` but not `COUNT({field})`; a ranked \
                 count axis counts documents per group, which is what `COUNT(*)` means. \
                 Drop the field to rank by group size."
            ))));
        }
        (SelectFunction::Sum, "") | (SelectFunction::Avg, "") => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(
                "`SUM` / `AVG` ranked queries require a non-empty select field naming the \
                 index's `summable` property"
                    .to_string(),
            )));
        }
        (SelectFunction::Sum, field) => (RankedAxis::Sum, field.to_string()),
        (SelectFunction::Avg, field) => (RankedAxis::Avg, field.to_string()),
        (other, _) => {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "ranked queries support `COUNT(*)`, `SUM(field)` and `AVG(field)` \
                 selects; got {other:?}. Ranking is driven by an indexed tree's \
                 per-axis secondary, and grovedb maintains exactly those three axes."
            ))));
        }
    };

    // ---- ORDER BY: exactly one clause, naming the selected aggregate -
    //
    // This clause *is* the ranking. One clause, because the secondary
    // is a single-key ordering: a second sort key would need a second
    // axis the storage does not maintain (ties are already resolved,
    // by group key, as a property of the directional scan).
    let expected_order_key = ranked_order_key(select);
    let order_clause = match order_by {
        [only] => only,
        _ => {
            return Err(Error::Query(QuerySyntaxError::InvalidParameter(format!(
                "ranked queries require exactly one `order_by` clause — the one that names \
                 the selected aggregate (`ORDER BY {expected_order_key}`); got {}. The axis \
                 secondary is a single-key ordering, so there is no second sort key to \
                 apply; ties are broken by group key in the direction of the walk.",
                order_by.len()
            ))));
        }
    };
    if order_clause.field != expected_order_key {
        return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "`GROUP BY {group_by_property} ORDER BY {}` is not supported: with a `GROUP BY` \
             present, the only ordering the ranked storage can serve is one on the selected \
             aggregate itself — write `ORDER BY {expected_order_key}` for \
             `SELECT {:?}({})`{}. Ordering groups by anything else (a raw document property, \
             or a second aggregate) would need a sort the axis secondary does not maintain.",
            order_clause.field,
            select.function,
            if select.field.is_empty() {
                "*"
            } else {
                select.field.as_str()
            },
            if axis == RankedAxis::Count {
                format!(
                    " (`{RANKED_COUNT_ORDER_KEY}` is the sentinel for COUNT(*), which has no \
                     field of its own; the `$` prefix is DPP's system namespace so it cannot \
                     collide with a schema property)"
                )
            } else {
                String::new()
            }
        ))));
    }
    // `ORDER BY <agg> DESC` is the "highest first" reading, so
    // descending is the negation of the clause's ascending flag.
    let descending = !order_clause.ascending;

    // ---- HAVING: must be absent -------------------------------------
    //
    // `having` is a boolean per-group predicate. Composing one with an
    // aggregate ordering means "rank only the groups that pass the
    // filter", and the axis secondary cannot do that: it is ordered by
    // aggregate, and there is no way to skip non-matching groups
    // without walking (and proving) every one of them, which destroys
    // the O(log n + k) bound the surface exists for.
    if !having.is_empty() {
        return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "boolean `having` cannot yet be combined with an aggregate `order_by`: got {} \
             having clause(s) alongside `ORDER BY {expected_order_key}`. Ranking reads a \
             pre-sorted axis secondary; filtering groups first would require walking every \
             group to test the predicate, which is exactly the cost the ranked surface \
             exists to avoid. Drop the `having`, or use a non-ranked grouped aggregate.",
            having.len()
        ))));
    }

    // ---- WHERE: equality pins on the compound prefix ------------------
    //
    // Empty for the single-property form. For a compound ranked index,
    // each `where` clause must pin one leading index property with `==`
    // — that is what selects which prefix's secondary the walk reads
    // (per-prefix semantics: there is no global cross-prefix ordering to
    // serve). Anything other than a distinct-property equality is
    // rejected loudly here; whether the pinned set matches a covering
    // index's leading properties exactly is the index picker's call.
    let prefix_pins = prefix_pins_from_where_clauses(where_clauses)?;

    // ---- LIMIT: required, 1 ..= MAX_RANKED_LIMIT ---------------------
    //
    // Required rather than defaulted: `k` is echoed inside the proof
    // envelope and re-checked by the verifier, so a server-chosen
    // default would be a number the client never agreed to and could
    // not reproduce when rebuilding the query to verify.
    let limit = pagination.limit.ok_or_else(|| {
        Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "ranked queries require an explicit `limit` (1 ..= {MAX_RANKED_LIMIT}): it is \
             the number of groups the walk returns, and it is echoed in the proof envelope \
             and re-checked by the verifier, so there is no server-side default a client \
             could reproduce. Write `ORDER BY {expected_order_key} DESC LIMIT 1` for the \
             single best-ranked group."
        )))
    })?;
    if limit == 0 {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "`LIMIT 0` selects nothing; ranked queries require 1 ≤ limit ≤ {MAX_RANKED_LIMIT}"
        ))));
    }
    if limit > MAX_RANKED_LIMIT as u32 {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
            "`LIMIT {limit}` exceeds the ranked-query ceiling of {MAX_RANKED_LIMIT}; the \
             proof commits one secondary entry per returned group, so its size grows \
             linearly in the limit. Narrow the request — the ceiling is a hard limit, not \
             a clamp, because `k` is echoed in the proof envelope and re-checked by the \
             verifier. Deep results are reached with `OFFSET`, whose skip work is bounded by \
             tree depth and does not grow with the offset."
        ))));
    }
    // Bounded by MAX_RANKED_LIMIT (a u16) immediately above.
    let k = limit as u16;

    // ---- OFFSET: optional, unbounded --------------------------------
    //
    // No ceiling, and that is a deliberate statement about cost rather
    // than an oversight. grovedb skips by *counting*, not by walking:
    // it descends the secondary reading each subtree's aggregate count
    // and collapses any subtree that fits inside the remaining offset,
    // so `OFFSET 4` and `OFFSET 4_000_000_000` are the same order of
    // O(log n + k) work — the deeper one in fact cheaper, since a tree
    // that fits entirely inside the offset collapses at the root.
    //
    // Both executors get that: the prover attests the skipped region
    // from the counted subtree commitments, and the unproved read
    // performs the same counted descent without building a proof. So
    // there is no denial-of-service lever here for a cap to close on
    // either path, and an arbitrary cap would only break honest deep
    // pagination. An offset past the end is a real answer (empty page,
    // `skipped` reporting the population), not an error.
    let offset = pagination.offset.unwrap_or(0);

    // ---- START AT: must be absent -----------------------------------
    if pagination.has_start_at {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(
            "ranked queries do not accept `start_at` / `start_after`: the cursor names a \
             document id, but a ranked walk iterates an aggregate-ordered keyspace in \
             which document ids do not appear. Paginate with `OFFSET` instead — it is \
             O(log n + k) at any depth."
                .to_string(),
        )));
    }

    // ---- OFFSET × IN: mutually exclusive -----------------------------
    //
    // Rank-skip is served from counted subtree commitments *inside one
    // secondary*; there is no counted structure spanning a branch
    // union, so a cross-branch offset would have to walk (and prove)
    // the skipped region in every branch — silently expensive. Callers
    // who need deep pages issue per-prefix requests, where offset works
    // exactly as documented.
    if offset > 0 && prefix_pins.iter().any(|pin| pin.values.len() > 1) {
        return Err(Error::Query(QuerySyntaxError::InvalidLimit(
            "`OFFSET` cannot combine with an `IN` prefix pin: rank-skip is attested from \
             one secondary's counted commitments, and an `IN` merges several secondaries \
             with no counted structure over the union. Page one prefix at a time (`==` \
             pin + `OFFSET`), or drop the offset."
                .to_string(),
        )));
    }

    Ok(DocumentRankedMode {
        axis,
        descending,
        k,
        offset,
        group_by_property,
        aggregate_field,
        prefix_pins,
    })
}
