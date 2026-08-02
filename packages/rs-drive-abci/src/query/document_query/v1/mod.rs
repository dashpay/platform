//! v1 handler for `getDocuments` — SQL-shaped unified surface
//! covering both matched-document queries and count queries under a
//! single request type with `select`, `group_by`, and `having`
//! clauses.
//!
//! ## What this handler is
//!
//! **Wire-format unification.** Every supported request shape
//! reaches an existing drive executor (`DriveDocumentQuery` for
//! `DOCUMENTS`, `Drive::execute_document_count_request` for
//! `COUNT`) and produces the same proof bytes / response data
//! the now-removed `getDocumentsCount` v0 endpoint did. The v1
//! surface just makes the SQL semantics explicit on the wire so
//! callers don't have to reverse-engineer "this where clause
//! shape happens to produce per-value entries."
//!
//! ## What it rejects
//!
//! Every request shape outside the existing drive-executor surface
//! returns [`QuerySyntaxError::Unsupported`] with `"… is not yet
//! implemented"` text. The error variant carries a `String` so the
//! exact rejected shape reaches the caller, and the message wording
//! signals **future capability**, not malformed request — clients
//! can keep these requests around in code and they'll start working
//! once the capability lands without a wire-format change. See the
//! message-level docstring on `GetDocumentsRequestV1` in
//! `platform.proto` for the full supported / rejected shape table.

mod compute_aggregate_mode_and_check_limit;
mod conversions;

use self::compute_aggregate_mode_and_check_limit::{
    compute_aggregate_mode_and_check_limit, AggregateRouting,
};

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start as RequestV0Start;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Start as RequestV1Start;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1;
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    average_results, count_results, ranked_entry, result_data, sum_results, AverageAggregate,
    AverageEntries, AverageEntry, AverageResults, CountEntries, CountEntry, CountResults,
    Documents, RankedEntries, RankedEntry, ResultData, SumEntries, SumEntry, SumResults,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{
    AverageEntry as DriveAverageEntry, AverageMode, CountMode, DocumentAverageRequest,
    DocumentAverageResponse, DocumentCountRequest, DocumentCountResponse, DocumentRankedRequest,
    DocumentRankedResponse, DocumentSumRequest, DocumentSumResponse, HavingClause, OrderClause,
    RankedEntry as DriveRankedEntry, RankedEntryValue, SelectFunction, SelectProjection,
    SplitCountEntry, SumEntry as DriveSumEntry, SumMode, WhereClause,
};
use drive::util::grove_operations::GroveDBToUse;

/// Build a `QuerySyntaxError::Unsupported` carrying a stable
/// "<feature> is not yet implemented" message. The wording is
/// deliberate — v1 publishes a SQL-shaped surface that the server
/// only partially implements today; the rejected shapes signal
/// future capability, not malformed requests, and callers can keep
/// the request structure unchanged when the capability lands.
pub(super) fn not_yet_implemented(feature: &str) -> QueryError {
    QueryError::Query(QuerySyntaxError::Unsupported(format!(
        "{} is not yet implemented",
        feature
    )))
}

/// Validate the `select` × `group_by` × `having` combination
/// against the supported-shape table (see the message-level
/// docstring on `GetDocumentsRequestV1` in `platform.proto`).
/// Returns the routing decision so the handler knows whether to
/// dispatch to the documents-fetch path or the count path, and
/// which response shape to produce.
fn validate_and_route(
    select: &SelectProjection,
    limit: Option<u32>,
    having: &[HavingClause],
    group_by: &[String],
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
    // it constrains (or, for a ranking operand, orders) the groups a
    // `COUNT` / `SUM` / `AVG` produces. Those three functions route
    // through the versioned `compute_aggregate_mode_and_check_limit`
    // helper below, which is where the ranked gate lives — under the
    // v0 table every non-empty HAVING is still rejected, under v1 a
    // ranking operand routes to the ranked executor.
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
                group_by,
                where_clauses,
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
                group_by,
                where_clauses,
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
            }
        }
        SelectFunction::Min => Err(not_yet_implemented(
            "SELECT MIN (the wire surface accepts MIN(field) so callers \
             can encode it ahead of server support landing, but the \
             server doesn't yet evaluate per-group MIN; semantically \
             distinct from `HavingRanking::Min` which is a cross-group \
             ranking primitive)",
        )),
        SelectFunction::Max => Err(not_yet_implemented(
            "SELECT MAX (the wire surface accepts MAX(field) so callers \
             can encode it ahead of server support landing, but the \
             server doesn't yet evaluate per-group MAX; semantically \
             distinct from `HavingRanking::Max` which is a cross-group \
             ranking primitive)",
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
                group_by,
                where_clauses,
                limit,
                having,
                "COUNT",
                platform_version,
            )? {
                AggregateRouting::Grouped(mode) => Ok(RoutingDecision::Count(mode)),
                AggregateRouting::Ranked => Ok(RoutingDecision::Ranked),
            }
        }
    }
}

/// Outcome of `validate_and_route` — names the path the v1 request
/// will dispatch to.
///
/// `Count(CountMode)` carries the SQL-shape contract (`Aggregate` /
/// `GroupByIn` / `GroupByRange` / `GroupByCompound`) directly; the
/// dispatcher passes it through to [`DocumentCountRequest::mode`]
/// without further translation.
enum RoutingDecision {
    Documents,
    Count(CountMode),
    /// `SELECT SUM(field)` routing. `sum_property` is the integer
    /// property to aggregate; the dispatcher in rs-drive will
    /// validate that it matches the doctype's `documents_summable`
    /// or a covering index's `summable: "<field>"`. `mode` mirrors
    /// `Count(mode)` — `SumMode` and `CountMode` are isomorphic
    /// enums sharing the same four variants. The response path
    /// emits the `SumResults` proto message added to platform.proto.
    Sum {
        sum_property: String,
        mode: CountMode,
    },
    /// `SELECT AVG(field)` routing. Same field rules as `Sum` —
    /// averages reuse sum-tree indexes and return a `(count, sum)`
    /// pair the client divides. `mode` carries the same shape as
    /// `Count` / `Sum`. The response path emits the
    /// `AverageResults` proto message added to platform.proto.
    Average {
        sum_property: String,
        mode: CountMode,
    },
    /// Ranked routing: a `COUNT` / `SUM` / `AVG` select whose single
    /// `HAVING` clause carries a ranking right operand. Routing does
    /// not read the ranking kind: `MAX` / `MIN` route here too and are
    /// then refused by drive, which owns that judgement. Dispatches to
    /// [`Self::dispatch_ranked_v1`] → `Drive::execute_document_ranked_request`
    /// and emits the `RankedEntries` proto message.
    ///
    /// Carries nothing, unlike its `Sum` / `Average` siblings: those
    /// pass the projected property forward because their drive
    /// request takes it as a separate field, whereas the ranked drive
    /// request takes the whole `(select, group_by, having)` triple and
    /// resolves the axis, direction and `k` itself. The handler still
    /// owns all three after routing, so there is nothing to carry and
    /// no opportunity for the routing layer's reading of the ranking
    /// to drift from drive's.
    Ranked,
}

/// Test-only: expose the routing decision for unit tests without
/// needing a full `Platform` setup. Mirrors **both the rejection
/// messages and the gate ordering** of [`Platform::query_documents_v1`]
/// so a test that pins a first-fail message also pins the order
/// gates fire in, not just which gate eventually fires.
///
/// Sequence (same as the real handler at
/// [`Platform::query_documents_v1`]):
/// 1. `offset.is_some()` → `not_yet_implemented("OFFSET …")`
/// 2. `where_clauses_from_proto` → propagate `InvalidArgument` /
///    `Unsupported` decode errors
/// 3. `order_clauses_from_proto` → propagate aggregate-target
///    rejection / `InvalidArgument` decode errors
/// 4. `selects.len() > 1` → `not_yet_implemented("multi-projection …")`
/// 5. `select_from_proto` (first element, or default documents)
/// 6. `having_clauses_from_proto` → propagate `InvalidArgument`
///    decode errors (unknown aggregate-function / operator / ranking
///    discriminant, missing aggregate, missing right operand)
/// 7. [`validate_and_route`] — which itself runs `limit == Some(0)`
///    → the non-aggregate HAVING gate → per-function gates →
///    routing pick (including the versioned ranked gate).
///
/// Treats an unset `select` (proto-default) the same way the
/// handler does — as `SelectProjection::documents()`.
#[cfg(test)]
pub(super) fn validate_and_route_for_tests(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
    platform_version: &PlatformVersion,
) -> Result<&'static str, QueryError> {
    // 1. OFFSET pagination — rejected before any decoding.
    if request_v1.offset.is_some() {
        return Err(not_yet_implemented(
            "OFFSET pagination (use cursor pagination via `start_after` / \
             `start_at` instead)",
        ));
    }
    // 2. WHERE decoding — wire-malformed shapes (unknown operator
    //    discriminant, nested `DocumentFieldValue.list` beyond
    //    depth 1, …) reject as `InvalidArgument`. Runs even
    //    though the caller passes a separate pre-decoded
    //    `where_clauses` slice for the routing decision, because
    //    the depth-cap and similar decode-time contracts aren't
    //    exercisable otherwise.
    conversions::where_clauses_from_proto(request_v1.where_clauses.clone())?;
    // 3. ORDER BY decoding — aggregate-target reject as
    //    `Unsupported("ORDER BY on aggregate keys …")`.
    conversions::order_clauses_from_proto(request_v1.order_by.clone())?;
    // 4. Multi-projection SELECT rejection.
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
    // 5. Decode the single Select (or default to documents).
    let select = request_v1
        .selects
        .first()
        .cloned()
        .map(conversions::select_from_proto)
        .transpose()?
        .unwrap_or_else(SelectProjection::documents);
    // 6. HAVING decoding — wire-malformed clauses reject as
    //    `InvalidArgument` before any routing decision is taken.
    let having = conversions::having_clauses_from_proto(request_v1.having.clone())?;
    // 7. `validate_and_route` runs the inner `limit` / `having` /
    //    per-function gates.
    validate_and_route(
        &select,
        request_v1.limit,
        &having,
        &request_v1.group_by,
        where_clauses,
        platform_version,
    )
    .map(|d| match d {
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
    })
}

impl<C> Platform<C> {
    pub(super) fn query_documents_v1(
        &self,
        request_v1: GetDocumentsRequestV1,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        // Destructure the proto request once; the rest of the
        // pipeline consumes the individual fields by name.
        let GetDocumentsRequestV1 {
            data_contract_id,
            document_type,
            where_clauses: proto_where_clauses,
            order_by: proto_order_by,
            limit,
            start,
            prove,
            selects: proto_selects,
            group_by,
            having,
            offset,
        } = request_v1;

        // OFFSET pagination is not yet implemented — cursor
        // pagination via `start_after` / `start_at` is the
        // supported path today. Reject any non-None offset
        // before doing further work; same `not_yet_implemented`
        // contract as HAVING / SUM / AVG.
        if offset.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "OFFSET pagination (use cursor pagination via `start_after` / \
                 `start_at` instead)",
            )));
        }

        // Decode the proto-typed `repeated WhereClause` / `repeated
        // OrderClause` into drive's structured forms once, up
        // front. Both the routing decision and the downstream
        // executor consume the typed clauses directly — no CBOR
        // envelope on the v1 path.
        //
        // `having` is decoded here too, unconditionally. It used to
        // be short-circuited on `is_empty()` because the server
        // rejected non-empty HAVING wholesale and decoding just to
        // discard was pure overhead; now the ranked path consumes
        // the decoded clauses, so the decode runs for every request.
        // The consequence the old comment predicted follows
        // automatically: wire-malformed HAVING (bad discriminant,
        // missing aggregate, missing right operand, …) now surfaces
        // as `InvalidArgument` instead of being masked by the
        // blanket "not yet implemented" — including on protocol
        // versions whose routing table still refuses every HAVING,
        // since malformed input is malformed regardless of which
        // capability would have handled it.
        let where_clauses = match conversions::where_clauses_from_proto(proto_where_clauses) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };
        let order_by_clauses = match conversions::order_clauses_from_proto(proto_order_by) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };
        let having_clauses = match conversions::having_clauses_from_proto(having) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        // `selects` is `repeated Select` on the wire. Empty
        // list → default-construct a `documents()` projection
        // (keeps v0-style callers that don't opt into SELECT on
        // the documents path). `len > 1` is wire-only today —
        // multi-projection routing + response shape are deferred
        // to a follow-up; reject here with the standard
        // `not_yet_implemented` contract.
        if proto_selects.len() > 1 {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "multi-projection SELECT (the wire accepts `repeated Select` so \
                 callers can encode `SELECT COUNT(*), SUM(amount), AVG(rating)` \
                 ahead of server support landing, but today only single-projection \
                 requests are evaluated; the response shape will gain a parallel \
                 `repeated AggregateValue values` field when multi-projection \
                 lands)",
            )));
        }
        let select = match proto_selects.into_iter().next() {
            Some(s) => match conversions::select_from_proto(s) {
                Ok(s) => s,
                Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
            },
            None => SelectProjection::documents(),
        };

        let routing = match validate_and_route(
            &select,
            limit,
            &having_clauses,
            &group_by,
            &where_clauses,
            platform_version,
        ) {
            Ok(r) => r,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        match routing {
            RoutingDecision::Documents => self.dispatch_documents_v1(
                data_contract_id,
                document_type,
                where_clauses,
                order_by_clauses,
                limit,
                start,
                prove,
                platform_state,
                platform_version,
            ),
            RoutingDecision::Count(mode) => self.dispatch_count_v1(
                data_contract_id,
                document_type,
                where_clauses,
                order_by_clauses,
                limit,
                start,
                prove,
                mode,
                platform_state,
                platform_version,
            ),
            RoutingDecision::Sum { sum_property, mode } => self.dispatch_sum_v1(
                data_contract_id,
                document_type,
                where_clauses,
                order_by_clauses,
                limit,
                start,
                prove,
                sum_property,
                mode,
                platform_state,
                platform_version,
            ),
            RoutingDecision::Average { sum_property, mode } => self.dispatch_average_v1(
                data_contract_id,
                document_type,
                where_clauses,
                order_by_clauses,
                limit,
                start,
                prove,
                sum_property,
                mode,
                platform_state,
                platform_version,
            ),
            RoutingDecision::Ranked => self.dispatch_ranked_v1(
                data_contract_id,
                document_type,
                select,
                group_by,
                having_clauses,
                where_clauses,
                order_by_clauses,
                limit,
                start,
                prove,
                platform_state,
                platform_version,
            ),
        }
    }

    /// Dispatch a `select = SUM(field)` request to
    /// [`Drive::execute_document_sum_request`] and map the response
    /// into a `GetDocumentsResponseV1` carrying a `SumResults` payload
    /// (or a `Proof` payload when prove=true).
    ///
    /// Parallels [`Self::dispatch_count_v1`] line-by-line — same
    /// request construction, same error → typed-rejection mapping,
    /// same prove vs no-prove split. Only the response shape mapping
    /// differs: `DocumentSumResponse::Aggregate(i64)` →
    /// `SumResults::aggregate_sum`, `Entries(Vec<SumEntry>)` →
    /// `SumResults::entries`, `Proof(bytes)` → outer `result.proof`.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_sum_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        sum_property: String,
        mode: CountMode,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        if start.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "start_after / start_at with SELECT SUM (paginate by narrowing the \
                 range clause itself)",
            )));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let (_, contract_fetch_info) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let contract_fetch_info = check_validation_result_with_data!(contract_fetch_info.ok_or(
            QueryError::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            ))
        ));
        let contract_ref = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        // `SumMode` mirrors `CountMode` 1:1 — same four variants
        // computed via the same `compute_aggregate_mode_and_check_limit`
        // helper. Map across the isomorphism.
        let sum_mode = match mode {
            CountMode::Aggregate => SumMode::Aggregate,
            CountMode::GroupByIn => SumMode::GroupByIn,
            CountMode::GroupByRange => SumMode::GroupByRange,
            CountMode::GroupByCompound => SumMode::GroupByCompound,
        };

        let drive_request = DocumentSumRequest {
            contract: contract_ref,
            document_type,
            sum_property,
            where_clauses,
            order_clauses,
            mode: sum_mode,
            limit,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_sum_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentSumResponse::Aggregate(sum) => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Sums(SumResults {
                        variant: Some(sum_results::Variant::AggregateSum(sum)),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentSumResponse::Entries(entries) => {
                if sum_mode == SumMode::Aggregate {
                    // Mirror of count's same-arm: `select=SUM,
                    // group_by=[]` whose executor routed through a
                    // PerInValue path (In + no range + no prove)
                    // returns one entry per In branch. Fold them into
                    // a single aggregate. `checked_add` surfaces the
                    // narrow case where per-branch sums truly add to
                    // more than i64::MAX as a typed
                    // `QuerySyntaxError::Unsupported` rather than
                    // silently saturating at i64::MAX (which produces
                    // a deterministic-but-misleading answer).
                    let mut total: i64 = 0;
                    let mut overflow = false;
                    for e in &entries {
                        match total.checked_add(e.sum.unwrap_or(0)) {
                            Some(t) => total = t,
                            None => {
                                overflow = true;
                                break;
                            }
                        }
                    }
                    if overflow {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::Unsupported(
                                "aggregate SUM across In branches overflows i64 — \
                                 the In-fold cannot be represented; narrow the In set \
                                 or query branches individually"
                                    .to_string(),
                            ),
                        )));
                    }
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Sums(SumResults {
                                variant: Some(sum_results::Variant::AggregateSum(total)),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                } else {
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Sums(SumResults {
                                variant: Some(sum_results::Variant::Entries(SumEntries {
                                    entries: entries.into_iter().map(into_v1_sum_entry).collect(),
                                })),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                }
            }
            DocumentSumResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsResponseV1 {
                    result: Some(get_documents_response_v1::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }

    /// Dispatch a `select = AVG(field)` request to
    /// [`Drive::execute_document_average_request`] and map the response
    /// into a `GetDocumentsResponseV1` carrying an `AverageResults`
    /// payload (or a `Proof` payload when prove=true).
    ///
    /// Parallels [`Self::dispatch_sum_v1`] line-by-line — same request
    /// construction, same error → typed-rejection mapping, same prove
    /// vs no-prove split. The response shape mapping differs:
    /// `DocumentAverageResponse::Aggregate { count, sum }` →
    /// `AverageResults::aggregate_average`,
    /// `DocumentAverageResponse::Entries(_)` → `AverageResults::entries`,
    /// `DocumentAverageResponse::Proof(_)` → outer `result.proof`.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_average_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        sum_property: String,
        mode: CountMode,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        if start.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "start_after / start_at with SELECT AVG (paginate by narrowing the \
                 range clause itself)",
            )));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let (_, contract_fetch_info) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let contract_fetch_info = check_validation_result_with_data!(contract_fetch_info.ok_or(
            QueryError::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            ))
        ));
        let contract_ref = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        // `AverageMode` mirrors `CountMode` 1:1 — map across.
        let avg_mode = match mode {
            CountMode::Aggregate => AverageMode::Aggregate,
            CountMode::GroupByIn => AverageMode::GroupByIn,
            CountMode::GroupByRange => AverageMode::GroupByRange,
            CountMode::GroupByCompound => AverageMode::GroupByCompound,
        };

        let drive_request = DocumentAverageRequest {
            contract: contract_ref,
            document_type,
            sum_property,
            where_clauses,
            order_clauses,
            mode: avg_mode,
            limit,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_average_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentAverageResponse::Aggregate { count, sum } => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Averages(AverageResults {
                        variant: Some(average_results::Variant::AggregateAverage(
                            AverageAggregate { count, sum },
                        )),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentAverageResponse::Entries(entries) => {
                if avg_mode == AverageMode::Aggregate {
                    // Mirror sum-side's fold for the `select=AVG,
                    // group_by=[]` + PerInValue executor combo. Fold
                    // both count and sum across In branches. Either
                    // axis overflowing is surfaced as a typed
                    // `QuerySyntaxError::Unsupported` so the client
                    // doesn't get a silently-saturated answer to
                    // divide against (which would also misreport the
                    // average).
                    let mut total_count: u64 = 0;
                    let mut total_sum: i64 = 0;
                    let mut overflow_axis: Option<&'static str> = None;
                    for e in &entries {
                        match total_count.checked_add(e.count.unwrap_or(0)) {
                            Some(c) => total_count = c,
                            None => {
                                overflow_axis = Some("count");
                                break;
                            }
                        }
                        match total_sum.checked_add(e.sum.unwrap_or(0)) {
                            Some(s) => total_sum = s,
                            None => {
                                overflow_axis = Some("sum");
                                break;
                            }
                        }
                    }
                    if let Some(axis) = overflow_axis {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::Unsupported(format!(
                                "aggregate AVG across In branches overflows {axis} \
                                 ({} axis range); narrow the In set or query branches \
                                 individually",
                                if axis == "count" { "u64" } else { "i64" },
                            )),
                        )));
                    }
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Averages(AverageResults {
                                variant: Some(average_results::Variant::AggregateAverage(
                                    AverageAggregate {
                                        count: total_count,
                                        sum: total_sum,
                                    },
                                )),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                } else {
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Averages(AverageResults {
                                variant: Some(average_results::Variant::Entries(AverageEntries {
                                    entries: entries
                                        .into_iter()
                                        .map(into_v1_average_entry)
                                        .collect(),
                                })),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                }
            }
            DocumentAverageResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsResponseV1 {
                    result: Some(get_documents_response_v1::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }

    /// Forward a `select = DOCUMENTS` request through the shared
    /// `query_documents_typed` helper that v0 also dispatches into.
    /// v1 doesn't add any documents-side capability — the SQL-shaped
    /// fields (`select`, `group_by`, `having`) are all validated as
    /// documents-compatible above (empty `group_by`, empty `having`,
    /// etc.) before reaching here.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_documents_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type: String,
        where_clauses: Vec<WhereClause>,
        order_by_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        let start = start.map(|s| match s {
            RequestV1Start::StartAfter(b) => RequestV0Start::StartAfter(b),
            RequestV1Start::StartAt(b) => RequestV0Start::StartAt(b),
        });
        // `limit` is `optional uint32` on v1; the typed helper takes
        // `Option<u32>` directly (`None` → server default). `Some(0)`
        // can't reach here — `validate_and_route` rejects it for
        // every SELECT mode so the v1 contract is uniform; only
        // `None` or `Some(N > 0)` survive.
        let result = self.query_documents_typed(
            data_contract_id,
            document_type,
            where_clauses,
            order_by_clauses,
            limit,
            prove,
            start,
            platform_state,
            platform_version,
        )?;
        Ok(result.map(translate_documents_v0_to_v1))
    }

    /// Forward a `select = COUNT` request to drive's count
    /// dispatcher. `mode` is the SQL-shape contract derived from
    /// `(select, group_by, where)` by `validate_and_route`; drive
    /// uses it to pick the executor strategy and decide whether to
    /// collapse the response to a single aggregate or return per-
    /// group entries. The wire response is `GetDocumentsResponseV1`
    /// with the inner `ResultData.counts` variant for non-proof
    /// results.
    #[allow(clippy::too_many_arguments)]
    fn dispatch_count_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        mode: CountMode,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        if start.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "start_after / start_at with SELECT COUNT (paginate by narrowing the \
                 range clause itself)",
            )));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let (_, contract_fetch_info) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let contract_fetch_info = check_validation_result_with_data!(contract_fetch_info.ok_or(
            QueryError::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            ))
        ));
        let contract_ref = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let drive_request = DocumentCountRequest {
            contract: contract_ref,
            document_type,
            where_clauses,
            order_clauses,
            mode,
            limit,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_count_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentCountResponse::Aggregate(count) => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Counts(CountResults {
                        variant: Some(count_results::Variant::AggregateCount(count)),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentCountResponse::Entries(entries) => {
                if mode.is_aggregate() {
                    // `select=COUNT, group_by=[]` against a request
                    // that drove a PerInValue execution (In + no
                    // range + no prove). Sum entries into a single
                    // aggregate before emission. `checked_add`
                    // surfaces u64 overflow as a typed
                    // `QuerySyntaxError::Unsupported`; realistic
                    // ceiling is `|In| × max_per-branch-count` (well
                    // under u64), so triggering this path requires
                    // either a misconfigured count tree or an
                    // executor bug.
                    let mut total: u64 = 0;
                    let mut overflow = false;
                    for e in &entries {
                        // `count.unwrap_or(0)` here is safe: this
                        // arm is server-side, summing entries the
                        // executor emitted. Executor never emits
                        // `None` (that's an SDK-side
                        // synthesis-for-missing concept). The
                        // `unwrap_or(0)` is a belt-and-suspenders
                        // guard against any future executor that
                        // forgets the contract.
                        match total.checked_add(e.count.unwrap_or(0)) {
                            Some(t) => total = t,
                            None => {
                                overflow = true;
                                break;
                            }
                        }
                    }
                    if overflow {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::Unsupported(
                                "aggregate COUNT across In branches overflows u64 — \
                                 narrow the In set or query branches individually"
                                    .to_string(),
                            ),
                        )));
                    }
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Counts(CountResults {
                                variant: Some(count_results::Variant::AggregateCount(total)),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                } else {
                    GetDocumentsResponseV1 {
                        result: Some(get_documents_response_v1::Result::Data(ResultData {
                            variant: Some(result_data::Variant::Counts(CountResults {
                                variant: Some(count_results::Variant::Entries(CountEntries {
                                    entries: entries.into_iter().map(into_v1_entry).collect(),
                                })),
                            })),
                        })),
                        metadata: Some(
                            self.response_metadata_v0(platform_state, CheckpointUsed::Current),
                        ),
                    }
                }
            }
            DocumentCountResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsResponseV1 {
                    result: Some(get_documents_response_v1::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }

    /// Dispatch a ranked request — a `COUNT` / `SUM` / `AVG` select
    /// whose single `HAVING` clause carries a ranking (`TOP(n)` /
    /// `BOTTOM(n)`; `MAX` / `MIN` are forwarded too and refused by
    /// drive) — to
    /// [`Drive::execute_document_ranked_request`], and map the response
    /// into a `GetDocumentsResponseV1` carrying a `RankedEntries`
    /// payload (or a `Proof` payload when prove=true).
    ///
    /// Structurally parallel to [`Self::dispatch_count_v1`] — same
    /// contract fetch, same `Error::Query` → typed-rejection mapping,
    /// same prove vs no-prove split — with two differences worth
    /// naming:
    ///
    /// 1. **The request is forwarded whole.** `where_clauses`,
    ///    `limit` and `start` are passed down even though a ranked
    ///    request must carry none of them, because drive owns that
    ///    rejection: the SDK's client-side helpers call drive's
    ///    validator with no abci in the path, so re-checking here
    ///    would create a second, driftable copy of the grammar. The
    ///    rejections come back as `Error::Query(...)` and are
    ///    surfaced to the caller as query errors, not internal ones.
    ///    `order_by` is the sole exception — the drive request has no
    ///    field for it, so it is rejected here.
    /// 2. **Proving an empty ranking is mapped, not propagated.** See
    ///    [`empty_ranking_proof_rejection`].
    #[allow(clippy::too_many_arguments)]
    fn dispatch_ranked_v1(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        select: SelectProjection,
        group_by: Vec<String>,
        having: Vec<HavingClause>,
        where_clauses: Vec<WhereClause>,
        order_clauses: Vec<OrderClause>,
        limit: Option<u32>,
        start: Option<RequestV1Start>,
        prove: bool,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        // ORDER BY is refused here rather than downstream because
        // `DocumentRankedRequest` has no field to carry it, so drive
        // cannot own this rejection the way it owns `where` / `limit`
        // / `start_at`. The result order of a ranked query *is* the
        // ranking, produced by the direction of the walk over the
        // axis secondary: a caller-supplied ordering could only
        // agree with it (redundant) or contradict it (unsatisfiable —
        // re-sorting the k returned groups by a document property
        // answers a different question than the ranking asked).
        // Accepting and silently ignoring it is the one genuinely
        // dangerous option, so it is rejected loudly.
        if !order_clauses.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "ORDER BY is not valid for a ranked query: the entry order of a \
                     `HAVING … TOP(n)` / `BOTTOM(n)` result already is the ranking \
                     order (best-first for TOP, worst-first for BOTTOM). Drop \
                     `order_by`, or flip TOP ↔ BOTTOM to reverse the ranking."
                        .to_string(),
                ),
            ));
        }

        let contract_id: Identifier =
            check_validation_result_with_data!(data_contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let (_, contract_fetch_info) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let contract_fetch_info = check_validation_result_with_data!(contract_fetch_info.ok_or(
            QueryError::Query(QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            ))
        ));
        let contract_ref = &contract_fetch_info.contract;
        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let drive_request = DocumentRankedRequest {
            contract: contract_ref,
            document_type,
            group_by: &group_by,
            select,
            having: &having,
            where_clauses: &where_clauses,
            limit,
            // `offset` is rejected for every v1 request at the top of
            // `query_documents_v1`, so it can only be `None` here.
            offset: None,
            has_start_at: start.is_some(),
            prove,
        };

        let drive_response =
            match self
                .drive
                .execute_document_ranked_request(drive_request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => match empty_ranking_proof_rejection(&e) {
                    Some(rejection) => {
                        return Ok(QueryValidationResult::new_with_error(rejection));
                    }
                    None => return Err(e.into()),
                },
            };

        let response = match drive_response {
            DocumentRankedResponse::Entries(entries) => GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    // No aggregate-collapse arm here, unlike count /
                    // sum / average: a ranked result is always a list
                    // of groups. Even `TOP(1)` returns one *entry*,
                    // because the caller needs to know which group
                    // won, not only the winning value.
                    variant: Some(result_data::Variant::Ranked(RankedEntries {
                        // Order is preserved verbatim: entry order is
                        // the ranking order, and drive already
                        // asserted the list is no longer than `k`.
                        entries: entries.into_iter().map(into_v1_ranked_entry).collect(),
                    })),
                })),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            },
            DocumentRankedResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsResponseV1 {
                    result: Some(get_documents_response_v1::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

/// Translate an rs-drive `RankedEntry` into the wire `RankedEntry`.
/// Mirror of [`into_v1_entry`] / [`into_v1_sum_entry`] /
/// [`into_v1_average_entry`] for the ranked surface.
///
/// `key` passes through as the raw index-key bytes of the grouped
/// property value — the same bytes the proof commits to, so a client
/// verifying the proof compares byte-for-byte against what it
/// reconstructs.
///
/// The `value` oneof is always set: drive's `RankedEntryValue` has no
/// "absent" variant (unlike the count / sum entry types, whose
/// `Option` exists for the SDK's synthesize-for-missing-In-value
/// concept — a ranked result has no caller-supplied key set to be
/// silent about).
fn into_v1_ranked_entry(e: DriveRankedEntry) -> RankedEntry {
    RankedEntry {
        key: e.key,
        value: Some(match e.value {
            RankedEntryValue::Count(count) => ranked_entry::Value::Count(count),
            RankedEntryValue::Sum(sum) => ranked_entry::Value::Sum(sum),
            // 16-byte big-endian two's complement, the encoding the
            // proto field documents. `to_be_bytes` on `i128` is
            // exactly that, and is the inverse of the client's
            // `i128::from_be_bytes`.
            RankedEntryValue::AvgFixedPoint(avg) => {
                ranked_entry::Value::AvgFixedPoint(avg.to_be_bytes().to_vec())
            }
        }),
    }
}

/// Recognize the one grovedb failure that is a caller-facing
/// condition rather than a server fault: **an empty ranking cannot be
/// proved**.
///
/// grovedb's prover has no absence-proof shape for "this axis
/// secondary has no entries", so proving a ranking over an index that
/// holds no documents fails with a merk-level "Cannot create proof for
/// empty tree", wrapped by grovedb as `CorruptedData`. Reaching that
/// state needs nothing exotic — querying a freshly registered contract
/// with `prove = true` does it — so letting it propagate would answer
/// an ordinary request with an internal error (`Status::unknown`) and
/// an alarming server-side log line, and give the caller no idea that
/// the same request without `prove` succeeds and returns the empty
/// list.
///
/// Detection is by variant + marker substring rather than by a typed
/// error, because grovedb flattens the merk error into a
/// `CorruptedData(String)` at the indexed-axis proof boundary; the
/// substring is the merk-side constant. The match is deliberately
/// narrow: any other `CorruptedData` still propagates as an internal
/// error, because for every other cause that classification is
/// correct.
fn empty_ranking_proof_rejection(error: &drive::error::Error) -> Option<QueryError> {
    let drive::error::Error::GroveDB(grove_error) = error else {
        return None;
    };
    let drive::query::GroveError::CorruptedData(message) = grove_error.as_ref() else {
        return None;
    };
    if !message.contains("Cannot create proof for empty tree") {
        return None;
    }
    Some(QueryError::InvalidArgument(
        "this ranking has no groups yet, and an empty ranking cannot be proved: \
         grovedb has no absence-proof shape for an empty axis secondary. Retry \
         with `prove = false` — the unproven read answers the same request with \
         an empty entry list. Once the index holds at least one document, the \
         proved form works."
            .to_string(),
    ))
}

fn into_v1_entry(e: SplitCountEntry) -> CountEntry {
    CountEntry {
        in_key: e.in_key,
        key: e.key,
        // The wire `count` is `uint64`, so it can only carry
        // `Some(_)`. Server-side never emits `None` entries to
        // begin with — `None` is the SDK-side synthesis signal for
        // "caller's In array contained a value the proof was
        // silent on," and that decision lives client-side because
        // the wire never has the caller's full In array context.
        // `unwrap_or(0)` is defense-in-depth: a future executor
        // bug emitting `None` shouldn't crash the response path,
        // it should round to zero on the wire (matching the
        // proto's `uint64` default).
        count: e.count.unwrap_or(0),
    }
}

/// Translate an rs-drive `SumEntry` into the wire `SumEntry`. Mirror
/// of [`into_v1_entry`] for the sum surface.
fn into_v1_sum_entry(e: DriveSumEntry) -> SumEntry {
    SumEntry {
        in_key: e.in_key,
        key: e.key,
        // `sum` is `sint64` on the wire — same `None`-rounds-to-0
        // contract as `into_v1_entry`.
        sum: e.sum.unwrap_or(0),
    }
}

/// Translate an rs-drive `AverageEntry` into the wire `AverageEntry`.
/// Mirror of [`into_v1_entry`] + [`into_v1_sum_entry`] for the average
/// surface (carries both count and sum so the client can divide).
///
/// `zip_entries` in `drive_document_average_query::drive_dispatcher`
/// performs a strict two-pointer merge that errors out as
/// `CorruptedCodeExecution` on any per-`(in_key, key)` divergence
/// between the count and sum streams. So by the time an entry reaches
/// this mapper, both axes have already been asserted to agree on
/// `Some`-vs-`None` for the same key — meaning the dangerous
/// `(count: None, sum: Some(V))` bucket that could let a client
/// divide V by 0 cannot exist. The `unwrap_or(0)` below is therefore
/// defense-in-depth (same as [`into_v1_entry`] / [`into_v1_sum_entry`]
/// for individual count / sum entries) rather than load-bearing.
fn into_v1_average_entry(e: DriveAverageEntry) -> AverageEntry {
    AverageEntry {
        in_key: e.in_key,
        key: e.key,
        count: e.count.unwrap_or(0),
        sum: e.sum.unwrap_or(0),
    }
}

/// Translate a v0 `GetDocumentsResponseV0` into v1's response
/// envelope (Documents-or-Proof wrapping the v0 oneof result into
/// v1's `ResultData`-or-`Proof` shape).
fn translate_documents_v0_to_v1(
    response_v0: dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV0,
) -> GetDocumentsResponseV1 {
    let metadata = response_v0.metadata;
    let result = match response_v0.result {
        Some(get_documents_response_v0::Result::Documents(docs)) => {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Documents(Documents {
                    documents: docs.documents,
                })),
            }))
        }
        Some(get_documents_response_v0::Result::Proof(proof)) => {
            Some(get_documents_response_v1::Result::Proof(proof))
        }
        None => None,
    };
    GetDocumentsResponseV1 { result, metadata }
}

#[cfg(test)]
mod tests;
