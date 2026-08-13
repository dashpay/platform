//! Request-shape validation for the ranked query, and the versioned
//! `(select, group_by, order_by, limit, offset)` → [`DocumentRankedMode`]
//! resolution.
//!
//! Pure functions on the request shape — no Drive, no contract, no
//! indexes. Available under `server` (the dispatcher validates before
//! executing) and `verify` (the SDK validates the same way before
//! attempting proof verification), so both sides agree on which requests
//! are well-formed and on the `(axis, descending, k, offset)` tuple a
//! well-formed one resolves to. Index-dependent validation ("does an
//! index actually cover this axis?") needs the document type's index map
//! and lives in [`super::index_picker`].
//!
//! Versioned through
//! `platform_version.drive.methods.document.query.detect_ranked_mode`,
//! the same way
//! [`DriveDocumentCountQuery::detect_mode_versioned`](super::super::drive_document_count_query::DriveDocumentCountQuery::detect_mode_versioned)
//! routes count's table: the accepted request grammar is a consensus-
//! adjacent contract on the query surface, so relaxing it later has to
//! land behind a method-version bump rather than changing what an
//! already-deployed protocol version accepts.

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
use dpp::version::PlatformVersion;

/// Versioned entry point. Routes through
/// `platform_version.drive.methods.document.query.detect_ranked_mode`;
/// today only `0` is defined and maps to [`detect_ranked_mode_v0`]
/// verbatim.
pub fn detect_ranked_mode(
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
    order_by: &[OrderClause],
    where_clauses: &[WhereClause],
    pagination: RankedPaginationInputs,
    platform_version: &PlatformVersion,
) -> Result<DocumentRankedMode, Error> {
    match platform_version
        .drive
        .methods
        .document
        .query
        .detect_ranked_mode
    {
        0 => detect_ranked_mode_v0(
            select,
            group_by,
            having,
            order_by,
            where_clauses,
            pagination,
        ),
        version => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "detect_ranked_mode: unknown method version {version}; only 0 is supported"
        )))),
    }
}

/// The `ORDER BY` field name that names a given select's aggregate.
///
/// `SUM(f)` / `AVG(f)` are ordered by naming `f` — the same field the
/// projection aggregates, which is how SQL's `ORDER BY avg(grade)`
/// reads once the aggregate function is already fixed by the `SELECT`.
/// `COUNT(*)` has no field, so it is named by the
/// [`RANKED_COUNT_ORDER_KEY`] sentinel.
///
/// Public because request *builders* need it as much as the validator
/// does: an SDK offering `.order_by_selected_aggregate(…)` has to emit
/// the same string this function expects to read back, and a second
/// copy of the sentinel rule is a silent-rejection bug waiting for the
/// first `COUNT(*)` ranking.
pub fn ranked_order_key(select: &SelectProjection) -> &str {
    match select.function {
        SelectFunction::Count if select.field.is_empty() => RANKED_COUNT_ORDER_KEY,
        _ => select.field.as_str(),
    }
}

/// Translate a request's `where` clauses into equality pins —
/// `(property, value)` pairs, one per clause — for the ranked and
/// having-range surfaces.
///
/// Both surfaces read a compound index's per-prefix secondary by
/// descending through one prefix value tree per **leading** index
/// property, and only an equality clause names a single value tree to
/// descend into. So the grammar is: every `where` clause must be an
/// equality (`==`), each on a distinct property. `IN` is rejected
/// separately from the other operators because it *will* eventually be
/// serviceable (one branch per element, once multi-`IN` branching lands
/// on the document query surface) — the message says so — while a range
/// operator on a prefix property can never pin a single subtree.
///
/// Shape-only, like everything in this module: whether the pinned
/// properties are exactly the leading properties of a covering compound
/// index is the index picker's call.
pub fn equality_pins_from_where_clauses(
    where_clauses: &[WhereClause],
) -> Result<Vec<(String, Value)>, Error> {
    let mut pins: Vec<(String, Value)> = Vec::with_capacity(where_clauses.len());
    for clause in where_clauses {
        match clause.operator {
            WhereOperator::Equal => {}
            WhereOperator::In => {
                return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "`{} IN …` is not yet supported on a ranked / having-range query's \
                     prefix properties: each `IN` element names a different prefix value \
                     tree, so serving it means one secondary walk per element and a merged \
                     result — a future capability layered on multi-`IN` branching. Pin \
                     each leading index property with `==`, or issue one request per \
                     value.",
                    clause.field
                ))));
            }
            _ => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "a ranked / having-range query's `where` clauses must pin the covering \
                     compound index's leading properties with `==`: the per-prefix \
                     secondary lives under one prefix value tree per leading property, and \
                     only an equality names a single value tree to descend into — a range \
                     operator cannot pin a prefix",
                    ),
                ));
            }
        }
        if clause.field.is_empty() {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "a ranked / having-range query's `where` clause names an empty property",
                ),
            ));
        }
        if pins.iter().any(|(field, _)| field == &clause.field) {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "a ranked / having-range query pins the same property twice: each leading \
                 index property takes exactly one equality pin",
                ),
            ));
        }
        pins.push((clause.field.clone(), clause.value.clone()));
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
/// `m ≥ 0`. `WHERE` clauses, when present, must be **equality pins** on
/// distinct properties — one per leading property of a covering
/// compound ranked index (see
/// [`equality_pins_from_where_clauses`]); the ranking then reads that
/// pinned prefix's own secondary. With no `where` the covering index is
/// single-property, exactly as before.
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
    let equality_pins = equality_pins_from_where_clauses(where_clauses)?;

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
             verifier. Deep results are reached with `OFFSET`, which costs nothing."
        ))));
    }
    // Bounded by MAX_RANKED_LIMIT (a u16) immediately above.
    let k = limit as u16;

    // ---- OFFSET: optional, unbounded --------------------------------
    //
    // No ceiling, and that is a deliberate statement about cost rather
    // than an oversight: grovedb's paginated prover attests the skipped
    // region from the counted subtree commitments instead of walking
    // it, so proving `OFFSET 4` and `OFFSET 4_000_000_000` are the same
    // O(log n + k) work and the same proof size. There is no
    // denial-of-service lever here to cap, and an arbitrary cap would
    // only break honest deep pagination. An offset past the end is a
    // provable answer (empty page, `skipped` attesting the population),
    // not an error.
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

    Ok(DocumentRankedMode {
        axis,
        descending,
        k,
        offset,
        group_by_property,
        aggregate_field,
        equality_pins,
    })
}
