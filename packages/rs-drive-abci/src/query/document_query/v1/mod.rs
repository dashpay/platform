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
mod dispatch;
mod routing;

use drive::query::ResolvedTimeRange;
use routing::{reject_offset_off_the_ranked_path, validate_and_route};
// Re-exported so `tests.rs` (a `use super::*` consumer) keeps seeing
// the routing probe under its old name.
#[cfg(test)]
use routing::validate_and_route_for_tests;

// Names below are consumed only by `tests.rs`, which imports this
// module's scope wholesale via `use super::*`.
#[cfg(test)]
use {
    dapi_grpc::platform::v0::get_documents_response::get_documents_response_v0,
    dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
        self, count_results, ranked_entry, result_data, CountEntry, CountResults, RankedEntries,
        RankedEntry, ResultData,
    },
    dpp::data_contract::accessors::v0::DataContractV0Getters,
    dpp::identifier::Identifier,
    drive::query::WhereClause,
};

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1;
use dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV1;
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters as _;
use dpp::prelude::Identifier as ContractIdentifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::error::query::QuerySyntaxError;
use drive::query::{resolve_time_range_bucket_clause, CountMode, SelectProjection};
use std::sync::Arc;

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

/// The contract fetched once per request — by time-range resolution when
/// the request carries an `IN_TIME_RANGE` clause — and handed to the
/// aggregate dispatchers so they never fetch a second time.
pub(in crate::query::document_query::v1) type PrefetchedContract =
    Option<(ContractIdentifier, Arc<DataContractFetchInfo>)>;

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
    /// Ranked routing: a `COUNT` / `SUM` / `AVG` select with a
    /// `GROUP BY` whose single `ORDER BY` clause names the selected
    /// aggregate. Routing does not read the direction, the limit or the
    /// offset — those are drive's to resolve and to refuse. Dispatches
    /// to [`Self::dispatch_ranked_v1`] →
    /// `Drive::execute_document_ranked_request` and emits the
    /// `RankedEntries` proto message.
    ///
    /// Carries nothing, unlike its `Sum` / `Average` siblings: those
    /// pass the projected property forward because their drive
    /// request takes it as a separate field, whereas the ranked drive
    /// request takes the whole `(select, group_by, order_by, limit,
    /// offset)` set and resolves the axis, direction, `k` and skip
    /// itself. The handler still owns all of them after routing, so
    /// there is nothing to carry and no opportunity for the routing
    /// layer's reading of the ranking to drift from drive's.
    Ranked,
    /// Boolean-`HAVING` range routing: a `COUNT` / `SUM` / `AVG` select
    /// with a `GROUP BY` carrying exactly one `having` clause. Routing
    /// does not read the clause — whether it bounds the selected
    /// aggregate, whether its operator translates to a contiguous
    /// range, and the limit's contract are drive's to resolve and to
    /// refuse (`detect_having_mode`). Dispatches to
    /// [`Self::dispatch_having_v1`] →
    /// `Drive::execute_document_having_request` and emits the
    /// `RankedEntries` proto message with `skipped` unset (a range page
    /// has no rank base). Carries nothing, for the same reason `Ranked`
    /// carries nothing.
    HavingRange,
}

impl<C> Platform<C> {
    /// Parse the wire contract id and fetch the contract (through drive's
    /// cache) with the uniform error mapping every v1 document route uses.
    /// One fetch per request: time-range resolution and the aggregate
    /// dispatch arms both consume this instead of carrying their own
    /// copies of the parse → fetch → not-found sequence. The document-type
    /// lookup stays at each consumer — it borrows from the returned `Arc`.
    ///
    /// Outer `Err` is an internal error; inner `Err` is the validation
    /// error to hand back to the wire caller
    /// (`check_validation_result_with_data!` unwraps the nesting).
    #[allow(clippy::type_complexity)]
    pub(in crate::query::document_query::v1) fn fetch_contract_for_document_query_v1(
        &self,
        data_contract_id: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> Result<Result<(ContractIdentifier, Arc<DataContractFetchInfo>), QueryError>, Error> {
        let contract_id: ContractIdentifier = match data_contract_id.try_into() {
            Ok(id) => id,
            Err(_) => {
                return Ok(Err(QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )))
            }
        };
        let (_, contract_fetch_info) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;
        let Some(contract_fetch_info) = contract_fetch_info else {
            return Ok(Err(QueryError::Query(
                QuerySyntaxError::DataContractNotFound("contract not found for a document query"),
            )));
        };
        Ok(Ok((contract_id, contract_fetch_info)))
    }

    /// The contract an aggregate dispatch arm executes against: the
    /// time-range resolution's fetch when it ran, one fetch now
    /// otherwise. Called by each aggregate dispatcher AFTER its cheap
    /// shape guards, so their rejections keep precedence over
    /// contract-lookup errors. Same nested-`Result` contract as
    /// [`Self::fetch_contract_for_document_query_v1`].
    #[allow(clippy::type_complexity)]
    pub(in crate::query::document_query::v1) fn contract_for_aggregate_dispatch(
        &self,
        prefetched: PrefetchedContract,
        data_contract_id: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> Result<Result<(ContractIdentifier, Arc<DataContractFetchInfo>), QueryError>, Error> {
        match prefetched {
            Some(pair) => Ok(Ok(pair)),
            None => self.fetch_contract_for_document_query_v1(data_contract_id, platform_version),
        }
    }

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
            chained,
        } = request_v1;

        // Chained mode owns its own (deliberately narrow) shape and
        // routes before the SELECT machinery: the request's clauses
        // describe the INNER indexOnly query of a provable semi-join.
        if let Some(chained_join) = chained {
            return self.dispatch_chained_v1(
                data_contract_id,
                document_type,
                chained_join,
                proto_where_clauses,
                proto_order_by,
                limit,
                start,
                prove,
                proto_selects,
                group_by,
                having,
                offset,
                platform_state,
                platform_version,
            );
        }

        // NOTE: the OFFSET gate is no longer here. Whether an offset is
        // acceptable depends on where the request routes — the ranked
        // executor paginates by offset, nothing else does — so it runs
        // once `validate_and_route` has answered, in
        // `reject_offset_off_the_ranked_path`. Off the ranked path the
        // rejection is byte-identical to the one this block used to
        // emit, except on the having-range route, which gets its own
        // message (the legacy one recommends cursors that route also
        // rejects).

        // Decode the proto-typed `repeated WhereClause` / `repeated
        // OrderClause` into drive's structured forms once, up
        // front. Both the routing decision and the downstream
        // executor consume the typed clauses directly — no CBOR
        // envelope on the v1 path.
        //
        // `having` is decoded here too, unconditionally, even though
        // no protocol version evaluates it yet: wire-malformed HAVING
        // (bad discriminant, missing aggregate, missing right operand,
        // a retired `ranking` right operand) surfaces as
        // `InvalidArgument` rather than being masked by the blanket
        // "not yet implemented", since malformed input is malformed
        // regardless of which capability would have handled it.
        //
        // Time-range (IN_TIME_RANGE) clauses are partitioned out first. They
        // are resolved to concrete equality clauses on the bucketed source
        // field using the authoritative committed block time, so the rest of
        // the v1 pipeline (routing, executors, proofs) treats them as
        // ordinary equality lookups. The verifier re-derives the same bucket
        // from the quorum-signed response metadata time, so the proof
        // matches.
        // The `.any()` pre-check skips the two-Vec partition on the
        // overwhelmingly common clause set with no time-range operator.
        let (time_range_proto, normal_proto): (Vec<_>, Vec<_>) = if proto_where_clauses
            .iter()
            .any(conversions::is_time_range_clause)
        {
            proto_where_clauses
                .into_iter()
                .partition(conversions::is_time_range_clause)
        } else {
            (Vec::new(), proto_where_clauses)
        };

        let mut where_clauses = match conversions::where_clauses_from_proto(normal_proto) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };
        let mut resolved_time_ranges: Vec<ResolvedTimeRange> = Vec::new();
        // The contract fetched for time-range resolution, handed to the
        // aggregate dispatchers below so a request fetches at most once.
        let mut prefetched_contract: PrefetchedContract = None;

        if !time_range_proto.is_empty() {
            // LOAD-BEARING TIME SOURCE: the verifier re-derives the bucket
            // from the response metadata's `time_ms`, which
            // `response_metadata_v0` stamps from the state selected by
            // `CheckpointUsed`. Reading `platform_state` here matches that
            // only because every v1 document dispatch serves from
            // `GroveDBToUse::Current`. If any document route ever serves
            // from a checkpoint, this resolution must read the SAME state
            // the response metadata will be built from, or every time-range
            // proof will fail client-side verification.
            let block_time_ms =
                match platform_state.last_committed_block_time_ms() {
                    Some(t) => t,
                    None => return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::Unsupported(
                            "a time range (IN_TIME_RANGE) query requires a committed block time"
                                .to_string(),
                        ),
                    ))),
                };
            let (contract_id, contract_fetch_info) = check_validation_result_with_data!(self
                .fetch_contract_for_document_query_v1(
                    data_contract_id.clone(),
                    platform_version
                )?);
            let contract_ref = &contract_fetch_info.contract;
            let doc_type = check_validation_result_with_data!(contract_ref
                .document_type_for_name(document_type.as_str())
                .map_err(|_| QueryError::InvalidArgument(format!(
                    "document type {} not found for contract {}",
                    document_type, contract_id
                ))));
            for proto_wc in time_range_proto {
                let (field, selector, grid) =
                    match conversions::time_range_clause_from_proto(proto_wc) {
                        Ok(parsed) => parsed,
                        Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
                    };
                match resolve_time_range_bucket_clause(
                    &field,
                    selector,
                    grid,
                    doc_type,
                    block_time_ms,
                ) {
                    Ok((clause, resolved)) => {
                        where_clauses.push(clause);
                        // The resolved clause is an ordinary equality; only
                        // this list tells the executors that it must be
                        // matched against bucket starts of the resolved
                        // grid rather than raw timestamps (or another
                        // grid's starts), so it travels with the request.
                        resolved_time_ranges.push(resolved);
                    }
                    Err(drive::error::Error::Query(qe)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)))
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            prefetched_contract = Some((contract_id, contract_fetch_info));
        }
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
            &order_by_clauses,
            &where_clauses,
            platform_version,
        ) {
            Ok(r) => r,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        if let Err(e) = reject_offset_off_the_ranked_path(offset, &routing) {
            return Ok(QueryValidationResult::new_with_error(e));
        }

        match routing {
            // The documents route forwards the raw id into the shared
            // `query_documents_typed` helper (v0 dispatches into it too),
            // which owns its contract fetch; the aggregate arms below
            // consume the request's single fetch instead.
            RoutingDecision::Documents => self.dispatch_documents_v1(
                data_contract_id,
                document_type,
                where_clauses,
                resolved_time_ranges,
                order_by_clauses,
                limit,
                start,
                prove,
                platform_state,
                platform_version,
            ),
            RoutingDecision::Count(mode) => self.dispatch_count_v1(
                prefetched_contract,
                data_contract_id,
                document_type,
                where_clauses,
                resolved_time_ranges,
                order_by_clauses,
                limit,
                start,
                prove,
                mode,
                platform_state,
                platform_version,
            ),
            RoutingDecision::Sum { sum_property, mode } => self.dispatch_sum_v1(
                prefetched_contract,
                data_contract_id,
                document_type,
                where_clauses,
                resolved_time_ranges,
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
                prefetched_contract,
                data_contract_id,
                document_type,
                where_clauses,
                resolved_time_ranges,
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
                prefetched_contract,
                data_contract_id,
                document_type,
                select,
                group_by,
                having_clauses,
                where_clauses,
                resolved_time_ranges,
                order_by_clauses,
                limit,
                offset,
                start,
                prove,
                platform_state,
                platform_version,
            ),
            RoutingDecision::HavingRange => self.dispatch_having_v1(
                prefetched_contract,
                data_contract_id,
                document_type,
                select,
                group_by,
                having_clauses,
                where_clauses,
                resolved_time_ranges,
                order_by_clauses,
                limit,
                offset,
                start,
                prove,
                platform_state,
                platform_version,
            ),
        }
    }
}

#[cfg(test)]
mod tests;
