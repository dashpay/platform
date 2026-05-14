//! v1 handler for `getDocuments` — SQL-shaped unified surface
//! covering both matched-document queries and count queries under a
//! single request type with `select`, `group_by`, and `having`
//! clauses.
//!
//! ## What this handler is
//!
//! **Wire-format unification.** Phase 1 ships no new server-side
//! execution capability: every supported request shape reaches an
//! existing drive executor (`DriveDocumentQuery` for `DOCUMENTS`,
//! `Drive::execute_document_count_request` for `COUNT`) and produces
//! the same proof bytes / response data the now-removed
//! `getDocumentsCount` v0 endpoint did. The v1 surface just makes
//! the SQL semantics explicit on the wire so callers don't have to
//! reverse-engineer "this where clause shape happens to produce
//! per-value entries."
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
//! `platform.proto` for the full Phase 1 supported/rejected shape
//! table.

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start as RequestV0Start;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    Select, Start as RequestV1Start,
};
use dapi_grpc::platform::v0::get_documents_request::{
    GetDocumentsRequestV0, GetDocumentsRequestV1,
};
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    count_results, result_data, CountEntries, CountEntry, CountResults, Documents, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{
    CountMode, DocumentCountRequest, DocumentCountResponse, SplitCountEntry, WhereClause,
    WhereOperator,
};
use drive::util::grove_operations::GroveDBToUse;

/// Build a `QuerySyntaxError::Unsupported` carrying a stable
/// "<feature> is not yet implemented" message. The wording is
/// deliberate — Phase 1 of v1 publishes a SQL-shaped surface that
/// the server only partially implements; the rejected shapes signal
/// future capability, not malformed requests, and callers can keep
/// the request structure unchanged when the capability lands.
fn not_yet_implemented(feature: &str) -> QueryError {
    QueryError::Query(QuerySyntaxError::Unsupported(format!(
        "{} is not yet implemented",
        feature
    )))
}

/// Parse the raw CBOR-encoded `where` bytes into structured
/// [`WhereClause`]s. v1 needs the structured form to enforce
/// `group_by` ↔ where-field cross-checks before delegating.
fn decode_where_clauses(where_bytes: &[u8]) -> Result<Vec<WhereClause>, QueryError> {
    if where_bytes.is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = ciborium::de::from_reader(where_bytes).map_err(|_| {
        QueryError::Query(QuerySyntaxError::DeserializationError(
            "unable to decode 'where' query from cbor".to_string(),
        ))
    })?;
    let array = match value {
        Value::Array(a) => a,
        Value::Null => return Ok(Vec::new()),
        _ => {
            return Err(QueryError::Query(
                QuerySyntaxError::InvalidFormatWhereClause(
                    "where clause must be an array".to_string(),
                ),
            ));
        }
    };
    let mut clauses = Vec::with_capacity(array.len());
    for entry in array {
        let components = match entry {
            Value::Array(c) => c,
            _ => {
                return Err(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause(
                        "where clause must be an array".to_string(),
                    ),
                ));
            }
        };
        let clause = WhereClause::from_components(&components).map_err(|e| {
            QueryError::Query(QuerySyntaxError::InvalidFormatWhereClause(format!(
                "invalid where clause components: {e}"
            )))
        })?;
        clauses.push(clause);
    }
    Ok(clauses)
}

/// Re-decode the CBOR-encoded `order_by` bytes into a `Value` for
/// drive's count dispatcher (which accepts the raw `Value` form to
/// avoid re-imposing a parse). `Value::Null` (empty `order_by` on
/// the wire) → no clauses.
fn decode_order_by_value(order_by_bytes: &[u8]) -> Result<Value, QueryError> {
    if order_by_bytes.is_empty() {
        return Ok(Value::Null);
    }
    ciborium::de::from_reader(order_by_bytes).map_err(|_| {
        QueryError::Query(QuerySyntaxError::DeserializationError(
            "unable to decode 'order_by' query from cbor".to_string(),
        ))
    })
}

/// Validate the `select` × `group_by` × `having` combination
/// against the Phase 1 supported-shape table. Returns the routing
/// decision so the handler knows whether to dispatch to the
/// documents-fetch path or the count path, and which response
/// shape to produce.
fn validate_and_route(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
) -> Result<RoutingDecision, QueryError> {
    let select = Select::try_from(request_v1.select).map_err(|_| {
        not_yet_implemented(&format!(
            "select value {} (not in the Select enum)",
            request_v1.select
        ))
    })?;

    if !request_v1.having.is_empty() {
        return Err(not_yet_implemented("HAVING clause"));
    }

    match select {
        Select::Documents => {
            if !request_v1.group_by.is_empty() {
                return Err(not_yet_implemented(
                    "GROUP BY with SELECT DOCUMENTS (use SELECT COUNT with GROUP BY \
                     for per-group counts, or SELECT DOCUMENTS without GROUP BY for \
                     matched documents)",
                ));
            }
            Ok(RoutingDecision::Documents)
        }
        Select::Count => {
            let in_field: Option<&str> = where_clauses
                .iter()
                .find(|wc| wc.operator == WhereOperator::In)
                .map(|wc| wc.field.as_str());
            let range_field: Option<&str> = where_clauses
                .iter()
                .find(|wc| {
                    matches!(
                        wc.operator,
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
                })
                .map(|wc| wc.field.as_str());

            // Compute the SQL-shape mode from `(group_by, where)`
            // first; check `limit` validity against the mode after
            // so the rejection lives in one place keyed off
            // `CountMode::accepts_limit()`.
            let mode = match request_v1.group_by.as_slice() {
                [] => CountMode::Aggregate,
                [field] => {
                    if Some(field.as_str()) == in_field {
                        // Single-field GROUP BY on the `In` field is
                        // only well-defined when no range clause is
                        // also constraining the result; otherwise
                        // Drive's compound walk emits unmerged
                        // `(in_key, key)` entries that don't match
                        // the caller's stated grouping. Force them
                        // to spell out the compound shape with a
                        // two-element `group_by`.
                        if range_field.is_some() {
                            return Err(not_yet_implemented(
                                "single-field GROUP BY when both `In` and range \
                                 clauses are present (use a two-element GROUP BY \
                                 `[in_field, range_field]` for the compound shape, \
                                 or drop the other constraint)",
                            ));
                        }
                        CountMode::GroupByIn
                    } else if Some(field.as_str()) == range_field {
                        // Same compound-shape concern as the In
                        // branch above — `group_by=[range_field]`
                        // with an active `In` clause produces
                        // compound rows from Drive that don't match
                        // the caller's grouping.
                        if in_field.is_some() {
                            return Err(not_yet_implemented(
                                "single-field GROUP BY when both `In` and range \
                                 clauses are present (use a two-element GROUP BY \
                                 `[in_field, range_field]` for the compound shape, \
                                 or drop the other constraint)",
                            ));
                        }
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
                    if Some(first.as_str()) == in_field && Some(second.as_str()) == range_field {
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
            if request_v1.limit.is_some() && !mode.accepts_limit() {
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
/// needing a full `Platform` setup.
#[cfg(test)]
pub(super) fn validate_and_route_for_tests(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
) -> Result<&'static str, QueryError> {
    validate_and_route(request_v1, where_clauses).map(|d| match d {
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
        let where_clauses = match decode_where_clauses(&request_v1.r#where) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let routing = match validate_and_route(&request_v1, &where_clauses) {
            Ok(r) => r,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        match routing {
            RoutingDecision::Documents => {
                self.dispatch_documents_v1(request_v1, platform_state, platform_version)
            }
            RoutingDecision::Count(mode) => {
                self.dispatch_count_v1(request_v1, mode, platform_state, platform_version)
            }
        }
    }

    /// Forward a `select = DOCUMENTS` request through the v0
    /// handler. v1 doesn't add any documents-side capability — the
    /// SQL-shaped fields (`select`, `group_by`, `having`) are all
    /// validated as documents-compatible above (empty `group_by`,
    /// empty `having`, etc.) before reaching here.
    fn dispatch_documents_v1(
        &self,
        request_v1: GetDocumentsRequestV1,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        let start = request_v1.start.map(|s| match s {
            RequestV1Start::StartAfter(b) => RequestV0Start::StartAfter(b),
            RequestV1Start::StartAt(b) => RequestV0Start::StartAt(b),
        });
        // `limit` is `optional uint32` on v1 vs unwrapped `uint32`
        // (default 0) on v0. Unset on v1 → 0 on v0 (v0 reads `0`
        // as "use the server's `default_query_limit`").
        let request_v0 = GetDocumentsRequestV0 {
            data_contract_id: request_v1.data_contract_id,
            document_type: request_v1.document_type,
            r#where: request_v1.r#where,
            order_by: request_v1.order_by,
            limit: request_v1.limit.unwrap_or(0),
            prove: request_v1.prove,
            start,
        };
        let result = self.query_documents_v0(request_v0, platform_state, platform_version)?;
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
    fn dispatch_count_v1(
        &self,
        request_v1: GetDocumentsRequestV1,
        mode: CountMode,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        if request_v1.start.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "start_after / start_at with SELECT COUNT (paginate by narrowing the \
                 range clause itself)",
            )));
        }

        let contract_id: Identifier = check_validation_result_with_data!(request_v1
            .data_contract_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument(
                "id must be a valid identifier (32 bytes long)".to_string()
            )));

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
            .document_type_for_name(request_v1.document_type.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                request_v1.document_type, contract_id
            ))));

        let where_value = if request_v1.r#where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(
                request_v1.r#where.as_slice()
            )
            .map_err(
                |_| QueryError::Query(QuerySyntaxError::DeserializationError(
                    "unable to decode 'where' query from cbor".to_string()
                ))
            ))
        };
        let order_by_value = match decode_order_by_value(&request_v1.order_by) {
            Ok(v) => v,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let drive_request = DocumentCountRequest {
            contract: contract_ref,
            document_type,
            raw_where_value: where_value,
            raw_order_by_value: order_by_value,
            mode,
            limit: request_v1.limit,
            prove: request_v1.prove,
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
