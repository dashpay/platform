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

mod conversions;

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
    count_results, result_data, CountEntries, CountEntry, CountResults, Documents, ResultData,
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
    CountMode, DocumentCountRequest, DocumentCountResponse, OrderClause, SelectFunction,
    SelectProjection, SplitCountEntry, WhereClause, WhereOperator,
};
use drive::util::grove_operations::GroveDBToUse;

/// Build a `QuerySyntaxError::Unsupported` carrying a stable
/// "<feature> is not yet implemented" message. The wording is
/// deliberate — v1 publishes a SQL-shaped surface that the server
/// only partially implements today; the rejected shapes signal
/// future capability, not malformed requests, and callers can keep
/// the request structure unchanged when the capability lands.
fn not_yet_implemented(feature: &str) -> QueryError {
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
    having_non_empty: bool,
    group_by: &[String],
    where_clauses: &[WhereClause],
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

    if having_non_empty {
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
                return Err(not_yet_implemented(
                    "GROUP BY with SELECT DOCUMENTS (use SELECT COUNT with GROUP BY \
                     for per-group counts, or SELECT DOCUMENTS without GROUP BY for \
                     matched documents)",
                ));
            }
            Ok(RoutingDecision::Documents)
        }
        SelectFunction::Sum => Err(not_yet_implemented(
            "SELECT SUM (the wire surface accepts SUM(field) so callers \
             can encode it ahead of server support landing, but the \
             server doesn't yet evaluate numeric aggregates other than \
             COUNT)",
        )),
        SelectFunction::Avg => Err(not_yet_implemented(
            "SELECT AVG (the wire surface accepts AVG(field) so callers \
             can encode it ahead of server support landing, but the \
             server doesn't yet evaluate numeric aggregates other than \
             COUNT)",
        )),
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
            let is_range_op = |op: WhereOperator| {
                matches!(
                    op,
                    WhereOperator::GreaterThan
                        | WhereOperator::GreaterThanOrEquals
                        | WhereOperator::LessThan
                        | WhereOperator::LessThanOrEquals
                        | WhereOperator::Between
                        | WhereOperator::BetweenExcludeBounds
                        | WhereOperator::BetweenExcludeLeft
                        | WhereOperator::BetweenExcludeRight
                        | WhereOperator::StartsWith
                )
            };
            let is_in_field = |field: &str| {
                where_clauses
                    .iter()
                    .any(|wc| wc.operator == WhereOperator::In && wc.field == field)
            };
            let is_range_field = |field: &str| {
                where_clauses
                    .iter()
                    .any(|wc| is_range_op(wc.operator) && wc.field == field)
            };

            // Compute the SQL-shape mode from `(group_by, where)`
            // first; check `limit` validity against the mode after
            // so the rejection lives in one place keyed off
            // `CountMode::accepts_limit()`.
            let mode = match group_by {
                [] => CountMode::Aggregate,
                [field] => {
                    if is_in_field(field) {
                        // Single-field GROUP BY on an `In`-constrained
                        // field routes to `CountMode::GroupByIn`.
                        // When a range clause is also present,
                        // drive's [`detect_mode`] picks the right
                        // submode — `RangeAggregateCarrierProof`
                        // on the prove path (one count per In
                        // branch via the grovedb #663 carrier
                        // primitive) or `RangeNoProof` on the
                        // no-prove path (per-In-branch entries
                        // from the range walk). Both produce
                        // entries that line up with the
                        // caller-stated GROUP BY shape, so no
                        // additional gating here is needed.
                        CountMode::GroupByIn
                    } else if is_range_field(field) {
                        // Symmetric to the In branch above:
                        // `group_by=[range_field]` routes to
                        // `CountMode::GroupByRange`. With a
                        // *second* range clause on a different
                        // field this drives the
                        // `RangeAggregateCarrierProof` carrier
                        // shape (drive's outer-range + inner-ACOR
                        // primitive). With an `In` on a different
                        // field it's `RangeDistinctProof` on the
                        // prove path (per-distinct-value counts
                        // with In-fanout on the prefix) or
                        // `RangeNoProof` distinct on the no-prove
                        // path.
                        CountMode::GroupByRange
                    } else {
                        return Err(not_yet_implemented(&format!(
                            "GROUP BY on field '{}' which is not constrained by an \
                             `In` or range where clause",
                            field
                        )));
                    }
                }
                [first, second] => {
                    if is_in_field(first) && is_range_field(second) {
                        CountMode::GroupByCompound
                    } else {
                        return Err(not_yet_implemented(
                            "two-field GROUP BY outside the `(In, range)` compound \
                             shape (the existing compound count path orders entries \
                             as `(in_key, key)`; other orderings would need a new \
                             merk walk)",
                        ));
                    }
                }
                _ => return Err(not_yet_implemented("GROUP BY with more than two fields")),
            };

            // Reject `limit` on modes that can't honor it. Aggregate
            // returns one row; GroupByIn is bounded by the In array
            // (capped at 100 by `WhereClause::in_values()`) and the
            // PointLookupProof path can't represent a partial-In
            // selection in its `SizedQuery`. Either way silent
            // truncation or fan-out summing would mislead callers
            // who set a `limit`.
            if limit.is_some() && !mode.accepts_limit() {
                let reason = match mode {
                    CountMode::Aggregate => {
                        "`limit` is not valid for SELECT COUNT with empty GROUP BY \
                         (aggregate count is a single row; omit `limit` to fix)"
                    }
                    CountMode::GroupByIn => {
                        "`limit` is not valid for SELECT COUNT with GROUP BY on an \
                         `In` field (result is bounded by the In array — capped at \
                         100 entries; narrow the In array directly to reduce the \
                         result set)"
                    }
                    CountMode::GroupByRange | CountMode::GroupByCompound => unreachable!(
                        "`accepts_limit()` returns true for these variants; \
                         outer guard already filtered them out"
                    ),
                };
                return Err(QueryError::Query(QuerySyntaxError::InvalidLimit(
                    reason.to_string(),
                )));
            }

            Ok(RoutingDecision::Count(mode))
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
}

/// Test-only: expose the routing decision for unit tests without
/// needing a full `Platform` setup. Mirrors the
/// `validate_and_route` argument shape so tests can drive it
/// directly with constructed field values.
///
/// Treats an unset `select` (proto-default) the same way the
/// handler does — as `SelectProjection::documents()`.
#[cfg(test)]
pub(super) fn validate_and_route_for_tests(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
) -> Result<&'static str, QueryError> {
    // OFFSET pagination is rejected before routing — mirror the
    // handler's gate here so tests can pin the contract.
    if request_v1.offset.is_some() {
        return Err(not_yet_implemented(
            "OFFSET pagination (use cursor pagination via `start_after` / \
             `start_at` instead)",
        ));
    }
    // Multi-projection SELECT is rejected before routing.
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
    // ORDER BY decoding can fail when the wire carries an
    // aggregate-target (rejected as `not_yet_implemented`); mirror
    // the handler's behavior so order-by-aggregate tests pin
    // through the test helper too.
    conversions::order_clauses_from_proto(request_v1.order_by.clone())?;
    // WHERE decoding can fail on wire-malformed input (unknown
    // operator discriminant, nested `DocumentFieldValue.list`
    // beyond depth 1, …). Run the decoder against the proto
    // `where_clauses` field even though the caller passes a
    // separate decoded `where_clauses` slice for the routing
    // decision — without this mirror the depth-cap and other
    // decode-time contracts aren't exercised through the test
    // helper.
    conversions::where_clauses_from_proto(request_v1.where_clauses.clone())?;
    let select = request_v1
        .selects
        .first()
        .cloned()
        .map(conversions::select_from_proto)
        .transpose()?
        .unwrap_or_else(SelectProjection::documents);
    validate_and_route(
        &select,
        request_v1.limit,
        !request_v1.having.is_empty(),
        &request_v1.group_by,
        where_clauses,
    )
    .map(|d| match d {
        RoutingDecision::Documents => "documents",
        RoutingDecision::Count(CountMode::Aggregate) => "count_aggregate",
        RoutingDecision::Count(CountMode::GroupByIn) => "count_entries_via_in_field",
        RoutingDecision::Count(CountMode::GroupByRange) => "count_entries_via_range_field",
        RoutingDecision::Count(CountMode::GroupByCompound) => "count_entries_via_compound",
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
        // `having` is checked for non-empty before decoding rather
        // than after: the server rejects non-empty HAVING
        // wholesale today, so decoding the clauses just to
        // discard them is pure overhead and the downstream
        // dispatchers don't accept the decoded vec yet. When
        // HAVING execution lands, the `is_empty()` short-circuit
        // gives way to a full `having_clauses_from_proto` call
        // that threads into the dispatchers — and at that point
        // wire-malformed HAVING (bad discriminant, missing
        // aggregate, …) starts surfacing as `InvalidArgument`
        // automatically.
        let where_clauses = match conversions::where_clauses_from_proto(proto_where_clauses) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };
        let order_by_clauses = match conversions::order_clauses_from_proto(proto_order_by) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };
        let having_non_empty = !having.is_empty();

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

        let routing =
            match validate_and_route(&select, limit, having_non_empty, &group_by, &where_clauses) {
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
        }
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
                    // aggregate before emission. `saturating_add`
                    // on the off-chance an operator-misconfigured
                    // count tree exceeds u64; realistic ceiling is
                    // `|In| × max_per-branch-count`, well under u64.
                    let total: u64 = entries
                        .iter()
                        // `count.unwrap_or(0)` here is safe: this
                        // arm is server-side, summing entries the
                        // executor emitted. Executor never emits
                        // `None` (that's an SDK-side
                        // synthesis-for-missing concept). The
                        // `unwrap_or(0)` is a belt-and-suspenders
                        // guard against any future executor that
                        // forgets the contract.
                        .map(|e| e.count.unwrap_or(0))
                        .fold(0u64, |a, b| a.saturating_add(b));
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
