//! Request-shape routing for the v1 `getDocuments` handler: the
//! `select` × `group_by` × `order_by` × `having` supported-shape
//! table, and the post-routing offset gate. Split from `mod.rs` for
//! readability — the logic is unchanged and unversioned on its own:
//! the routing *rules* version through the
//! `compute_aggregate_mode_and_check_limit` table it calls into.

use super::compute_aggregate_mode_and_check_limit::{
    compute_aggregate_mode_and_check_limit, AggregateRouting,
};
use super::{not_yet_implemented, RoutingDecision};
use crate::error::query::QueryError;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{HavingClause, OrderClause, SelectFunction, SelectProjection, WhereClause};
#[cfg(test)]
use {
    super::conversions, dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1,
    drive::query::CountMode,
};

/// Validate the `select` × `group_by` × `order_by` × `having`
/// combination against the supported-shape table (see the
/// message-level docstring on `GetDocumentsRequestV1` in
/// `platform.proto`). Returns the routing decision so the handler
/// knows whether to dispatch to the documents-fetch path, the count
/// path or the ranked path, and which response shape to produce.
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_and_route(
    select: &SelectProjection,
    limit: Option<u32>,
    having: &[HavingClause],
    group_by: &[String],
    order_by: &[OrderClause],
    where_clauses: &[WhereClause],
    platform_version: &PlatformVersion,
) -> Result<RoutingDecision, QueryError> {
    // Centralized `limit: Some(0)` rejection.
    //
    // `limit` is `optional uint32` on the wire, so `Some(0)` is a
    // distinct value any raw-gRPC/WASM/FFI caller can encode. Three
    // legacy behaviors collide on this value across the v1 dispatch
    // surface:
    // - `SELECT DOCUMENTS` would `unwrap_or(0)` and forward to v0,
    //   where `limit=0` is the v0-uint32 sentinel for "use server
    //   default" — accept-as-default.
    // - `SELECT COUNT` with `mode ∈ {Aggregate, GroupByIn}` would
    //   reject via the `is_some()` check below — reject-as-invalid.
    // - `SELECT COUNT` with `mode ∈ {GroupByRange, GroupByCompound}`
    //   would pass `Some(0)` through to drive, which honors it as a
    //   zero-cap walk — accept-as-zero.
    //
    // Three semantics for the same wire bytes is bad contract. The
    // v1 wire's whole point of switching to `optional uint32` was
    // to make "unset" explicit (`None`), so `Some(0)` only makes
    // sense as an *explicit* zero — and a zero-cap query returns
    // no useful information regardless of mode. Reject it uniformly
    // at the validation boundary so callers see a single,
    // mode-independent contract: `None` for "use server default",
    // `Some(N > 0)` for an explicit cap, `Some(0)` is invalid.
    if limit == Some(0) {
        return Err(QueryError::Query(QuerySyntaxError::InvalidLimit(
            "limit = 0 is not a valid wire value on the v1 \
             `optional uint32` field; omit `limit` (None) to use the \
             server's default, or pass a positive integer for an \
             explicit cap (a zero-cap query is structurally \
             meaningless regardless of SELECT mode)"
                .to_string(),
        )));
    }

    // HAVING is only ever meaningful for an aggregate projection:
    // it is a boolean predicate over the groups a `COUNT` / `SUM` /
    // `AVG` produces. Those three functions route through the
    // versioned `compute_aggregate_mode_and_check_limit` helper below,
    // whose v2 table routes a single grouped clause to the
    // having-range executor and whose older tables reject every
    // non-empty HAVING, with wording that depends on whether the
    // request is otherwise a ranked one.
    //
    // For every other SELECT there is no aggregate for a HAVING to
    // talk about, so the rejection stays here and stays unversioned.
    // It also stays *ahead* of the per-function gates below, exactly
    // where the old blanket rejection was: `SELECT DOCUMENTS … HAVING`
    // must keep reporting the HAVING rather than silently dropping it
    // on the documents path, and `SELECT MIN/MAX … HAVING` must not
    // start reporting MIN/MAX as the reason a request with two
    // unsupported features was refused.
    if !having.is_empty()
        && !matches!(
            select.function,
            SelectFunction::Count | SelectFunction::Sum | SelectFunction::Avg
        )
    {
        return Err(not_yet_implemented("HAVING clause"));
    }

    match select.function {
        SelectFunction::Documents => {
            if !select.field.is_empty() {
                return Err(QueryError::InvalidArgument(format!(
                    "SELECT DOCUMENTS does not accept a projection field; \
                     got field='{}' (omit the field for plain document fetch, \
                     or use SELECT COUNT / SUM / AVG to project a value)",
                    select.field
                )));
            }
            if !group_by.is_empty() {
                // GROUP BY with SELECT DOCUMENTS is structurally
                // nonsensical — GROUP BY produces one row per
                // distinct key, but SELECT DOCUMENTS returns the
                // underlying rows; the two contracts can't be
                // reconciled. Callers wanting per-group output use
                // SELECT COUNT / SUM / AVG / MIN / MAX. Classify
                // as `InvalidArgument` rather than
                // `not_yet_implemented` because this isn't a
                // future capability — no protocol version will
                // make this combination meaningful.
                return Err(QueryError::InvalidArgument(format!(
                    "GROUP BY with SELECT DOCUMENTS is not a valid SQL shape: \
                     GROUP BY produces one row per distinct key, but SELECT \
                     DOCUMENTS returns the underlying rows themselves. Use \
                     SELECT COUNT / SUM / AVG / MIN / MAX with GROUP BY for \
                     per-group output, or SELECT DOCUMENTS without GROUP BY \
                     for plain document fetch. Got group_by={:?}.",
                    group_by
                )));
            }
            Ok(RoutingDecision::Documents)
        }
        SelectFunction::Sum => {
            // SELECT SUM(field): routes to
            // `Drive::execute_document_sum_request` (in
            // `packages/rs-drive/src/query/drive_document_sum_query/`).
            // `field` must be non-empty and must name an integer
            // property on the document type that's covered by either
            // `documents_summable` (doctype level) or a `summable:
            // "<field>"` index. Validation lives downstream in
            // [`crate::query::drive_document_sum_query::drive_dispatcher::detect_sum_mode`].
            //
            // Wiring: `RoutingDecision::Sum(...)` variant below feeds
            // the dispatch arm in the response-building section, which
            // routes the resulting `DocumentSumResponse` into the
            // `SumResults` proto message defined in platform.proto.
            if select.field.is_empty() {
                return Err(QueryError::InvalidArgument(
                    "SELECT SUM requires a non-empty `field` naming the integer property \
                     to sum (e.g. `SUM(amount)`). The contract must declare \
                     `documentsSummable: \"<field>\"` at the document-type level OR a \
                     `summable: \"<field>\"` index covering the where-clause shape; the \
                     DPP validator enforces this at contract creation."
                        .to_string(),
                ));
            }
            match compute_aggregate_mode_and_check_limit(
                select,
                group_by,
                where_clauses,
                order_by,
                limit,
                having,
                "SUM",
                platform_version,
            )? {
                AggregateRouting::Grouped(mode) => Ok(RoutingDecision::Sum {
                    sum_property: select.field.clone(),
                    mode,
                }),
                AggregateRouting::Ranked => Ok(RoutingDecision::Ranked),
                AggregateRouting::HavingRange => Ok(RoutingDecision::HavingRange),
            }
        }
        SelectFunction::Avg => {
            // SELECT AVG(field): routes to
            // `Drive::execute_document_average_request` (in
            // `packages/rs-drive/src/query/drive_document_average_query/`).
            // `field` must be non-empty and must name an integer
            // property covered by either `documents_summable` (doctype
            // level) or a `summable: "<field>"` index — averages reuse
            // sum-tree indexes (no separate `averageable` flag exists
            // or is needed; the same `CountSumTree` / PCPS element
            // backs both).
            //
            // Wiring: `RoutingDecision::Average(...)` variant below
            // feeds the dispatch arm in the response-building section,
            // which routes the resulting `DocumentAverageResponse` into
            // the `AverageResults` proto message defined in
            // platform.proto.
            if select.field.is_empty() {
                return Err(QueryError::InvalidArgument(
                    "SELECT AVG requires a non-empty `field` naming the integer property \
                     to average (e.g. `AVG(score)`). The contract must declare \
                     `documentsSummable: \"<field>\"` at the document-type level OR a \
                     `summable: \"<field>\"` index covering the where-clause shape; the \
                     DPP validator enforces this at contract creation. Averages reuse \
                     sum-tree indexes — no separate `averageable` flag is required."
                        .to_string(),
                ));
            }
            match compute_aggregate_mode_and_check_limit(
                select,
                group_by,
                where_clauses,
                order_by,
                limit,
                having,
                "AVG",
                platform_version,
            )? {
                AggregateRouting::Grouped(mode) => Ok(RoutingDecision::Average {
                    sum_property: select.field.clone(),
                    mode,
                }),
                AggregateRouting::Ranked => Ok(RoutingDecision::Ranked),
                AggregateRouting::HavingRange => Ok(RoutingDecision::HavingRange),
            }
        }
        SelectFunction::Min => Err(not_yet_implemented(
            "SELECT MIN (the wire surface accepts MIN(field) so callers \
             can encode it ahead of server support landing, but the \
             server doesn't yet evaluate per-group MIN; semantically \
             distinct from asking for the lowest-ranked group, which is \
             `ORDER BY <the selected aggregate> ASC LIMIT 1`)",
        )),
        SelectFunction::Max => Err(not_yet_implemented(
            "SELECT MAX (the wire surface accepts MAX(field) so callers \
             can encode it ahead of server support landing, but the \
             server doesn't yet evaluate per-group MAX; semantically \
             distinct from asking for the highest-ranked group, which is \
             `ORDER BY <the selected aggregate> DESC LIMIT 1`)",
        )),
        SelectFunction::Count => {
            if !select.field.is_empty() {
                return Err(not_yet_implemented(
                    "SELECT COUNT(field) — counting non-null values of a \
                     specific field (the wire surface accepts the field so \
                     callers can encode it ahead of server support landing, \
                     but today only COUNT(*) — empty `field` — is evaluated)",
                ));
            }
            // Field-membership predicates on the request's where
            // clauses. **Match-any, not match-first** — a request
            // may carry two range clauses on different fields
            // (the executor's `RangeAggregateCarrierProof` path
            // is built for exactly that shape; see
            // `outer_range_plus_inner_range_with_prove_and_group_by_range_routes_to_carrier_proof`
            // in `drive/query/drive_document_count_query/tests.rs`).
            // A `find(...).map(field).map(eq)` test against a
            // hard-coded first range clause would make the routing
            // decision depend on clause ordering on the wire,
            // which is wrong — `WHERE a > x AND b > y GROUP BY a`
            // and `WHERE b > y AND a > x GROUP BY a` must produce
            // the same routing.
            //
            // For `In` the practical effect is the same because
            // `validate_and_canonicalize_where_clauses` rejects
            // multiple `In` clauses upstream (`MultipleInClauses`),
            // but the `any` shape is used here too so the routing
            // logic doesn't bake in an assumption that could go
            // stale if that validator's contract ever relaxes.
            match compute_aggregate_mode_and_check_limit(
                select,
                group_by,
                where_clauses,
                order_by,
                limit,
                having,
                "COUNT",
                platform_version,
            )? {
                AggregateRouting::Grouped(mode) => Ok(RoutingDecision::Count(mode)),
                AggregateRouting::Ranked => Ok(RoutingDecision::Ranked),
                AggregateRouting::HavingRange => Ok(RoutingDecision::HavingRange),
            }
        }
    }
}

/// The `OFFSET` gate, applied **after** routing.
///
/// Offset pagination exists on exactly one path: the ranked executor,
/// where `OFFSET m` is the rank the returned page starts at; skipping
/// is a counted tree descent on either `prove` setting — grovedb counts
/// the skipped region from the subtree aggregates rather than walking
/// it, so the work is bounded by tree depth and does not grow with `m`. Only the proved result
/// additionally *attests* the count. Every other v1 shape —
/// documents, and the grouped count / sum / average modes — has no
/// offset primitive behind it and keeps the rejection it has always
/// had, **message for message**: those callers paginate with
/// `start_after` / `start_at`, or by narrowing the range clause.
///
/// The legacy message below is load-bearing and must not be reworded:
/// clients match on it, and on a protocol version whose routing table
/// has no ranked path (v13 and earlier) it is the *only* answer an
/// offset can get, exactly as it was before the ranked surface existed.
///
/// The having-range route gets its own message instead, because the
/// legacy one gives that caller wrong advice: the having surface has
/// neither offset nor cursor pagination (`start_after` / `start_at`
/// are rejected by mode detection — a document-ID cursor cannot
/// address the aggregate-sorted secondary). The only continuation is
/// tightening the bound past the last distinct aggregate value, with
/// the documented tie limitation.
pub(super) fn reject_offset_off_the_ranked_path(
    offset: Option<u32>,
    decision: &RoutingDecision,
) -> Result<(), QueryError> {
    match decision {
        _ if offset.is_none() => Ok(()),
        RoutingDecision::Ranked => Ok(()),
        RoutingDecision::HavingRange => Err(not_yet_implemented(
            "OFFSET on a having-range query; this surface has no offset or cursor \
             pagination — to continue past a page cut at the limit, tighten the \
             `having` bound past the last aggregate value seen (this cannot cross \
             a tie: several groups sharing the boundary aggregate must fit inside \
             one limit)",
        )),
        _ => Err(not_yet_implemented(
            "OFFSET pagination (use cursor pagination via `start_after` / \
             `start_at` instead)",
        )),
    }
}

/// Test-only: expose the routing decision for unit tests without
/// needing a full `Platform` setup. Mirrors **both the rejection
/// messages and the gate ordering** of [`Platform::query_documents_v1`]
/// so a test that pins a first-fail message also pins the order
/// gates fire in, not just which gate eventually fires.
///
/// Sequence (same as the real handler at
/// [`Platform::query_documents_v1`]):
/// 1. `where_clauses_from_proto` → propagate `InvalidArgument` /
///    `Unsupported` decode errors
/// 2. `order_clauses_from_proto` → propagate aggregate-target
///    rejection / `InvalidArgument` decode errors
/// 3. `selects.len() > 1` → `not_yet_implemented("multi-projection …")`
/// 4. `select_from_proto` (first element, or default documents)
/// 5. `having_clauses_from_proto` → propagate `InvalidArgument`
///    decode errors (unknown aggregate-function / operator
///    discriminant, missing aggregate, missing / retired right
///    operand)
/// 6. [`validate_and_route`] — which itself runs `limit == Some(0)`
///    → the non-aggregate HAVING gate → per-function gates →
///    routing pick (including the versioned ranked gate).
/// 7. `offset.is_some()` on a non-ranked decision →
///    `not_yet_implemented("OFFSET …")`
///
/// The OFFSET gate is **last**, not first: whether an offset is
/// acceptable now depends on where the request routes (the ranked
/// executor paginates by offset; nothing else does), and that is not
/// known until routing has run. On a protocol version whose table has
/// no ranked path, every offset is still refused with the identical
/// message — only its position relative to the decode gates moved.
///
/// Treats an unset `select` (proto-default) the same way the
/// handler does — as `SelectProjection::documents()`.
#[cfg(test)]
pub(super) fn validate_and_route_for_tests(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
    platform_version: &PlatformVersion,
) -> Result<&'static str, QueryError> {
    // 1. WHERE decoding — wire-malformed shapes (unknown operator
    //    discriminant, nested `DocumentFieldValue.list` beyond
    //    depth 1, …) reject as `InvalidArgument`. Runs even
    //    though the caller passes a separate pre-decoded
    //    `where_clauses` slice for the routing decision, because
    //    the depth-cap and similar decode-time contracts aren't
    //    exercisable otherwise.
    conversions::where_clauses_from_proto(request_v1.where_clauses.clone())?;
    // 2. ORDER BY decoding — aggregate-target reject as
    //    `Unsupported("ORDER BY on aggregate keys …")`.
    let order_by_clauses = conversions::order_clauses_from_proto(request_v1.order_by.clone())?;
    // 3. Multi-projection SELECT rejection.
    if request_v1.selects.len() > 1 {
        return Err(not_yet_implemented(
            "multi-projection SELECT (the wire accepts `repeated Select` so \
             callers can encode `SELECT COUNT(*), SUM(amount), AVG(rating)` \
             ahead of server support landing, but today only single-projection \
             requests are evaluated; the response shape will gain a parallel \
             `repeated AggregateValue values` field when multi-projection \
             lands)",
        ));
    }
    // 4. Decode the single Select (or default to documents).
    let select = request_v1
        .selects
        .first()
        .cloned()
        .map(conversions::select_from_proto)
        .transpose()?
        .unwrap_or_else(SelectProjection::documents);
    // 5. HAVING decoding — wire-malformed clauses reject as
    //    `InvalidArgument` before any routing decision is taken.
    let having = conversions::having_clauses_from_proto(request_v1.having.clone())?;
    // 6. `validate_and_route` runs the inner `limit` / `having` /
    //    per-function gates.
    let decision = validate_and_route(
        &select,
        request_v1.limit,
        &having,
        &request_v1.group_by,
        &order_by_clauses,
        where_clauses,
        platform_version,
    )?;
    // 7. OFFSET, now that routing is known.
    reject_offset_off_the_ranked_path(request_v1.offset, &decision)?;
    Ok(match decision {
        RoutingDecision::Documents => "documents",
        RoutingDecision::Count(CountMode::Aggregate) => "count_aggregate",
        RoutingDecision::Count(CountMode::GroupByIn) => "count_entries_via_in_field",
        RoutingDecision::Count(CountMode::GroupByRange) => "count_entries_via_range_field",
        RoutingDecision::Count(CountMode::GroupByCompound) => "count_entries_via_compound",
        // v3 sum surface — single label for now (no sub-mode
        // breakdown like count's). `dispatch_sum_v1` further routes
        // by where-shape × prove flag.
        RoutingDecision::Sum { .. } => "sum",
        // v3 average surface — single label like sum;
        // `dispatch_average_v1` further routes by where-shape ×
        // prove flag once the executor lands.
        RoutingDecision::Average { .. } => "average",
        // Ranked surface — single label; the axis / direction / `k`
        // breakdown is drive's to resolve, not routing's, so there
        // is no sub-mode to report here.
        RoutingDecision::Ranked => "ranked",
        // Having-range surface — single label for the same reason as
        // ranked: the bounds / direction / limit breakdown is drive's.
        RoutingDecision::HavingRange => "having_range",
    })
}
