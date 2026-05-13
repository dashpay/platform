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
mod tests {
    //! Tests for the v1 `getDocuments` handler — pure wire-format
    //! unification of v0 documents + the (now-removed) v0-count
    //! endpoint.
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
        Select as V1Select, Start as V1Start,
    };
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::platform_value::platform_value;

    fn empty_v1_request() -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: false,
            select: V1Select::Documents as i32,
            group_by: Vec::new(),
            having: Vec::new(),
        }
    }

    fn assert_not_yet_implemented(
        result: Result<&'static str, QueryError>,
        expected_feature: &str,
    ) {
        match result {
            Err(QueryError::Query(QuerySyntaxError::Unsupported(msg))) => {
                assert!(
                    msg.contains(expected_feature) && msg.contains("not yet implemented"),
                    "expected message containing '{}' and 'not yet implemented', got: {}",
                    expected_feature,
                    msg
                );
            }
            other => panic!(
                "expected QueryError::Query(Unsupported) for '{}', got {:?}",
                expected_feature, other
            ),
        }
    }

    #[test]
    fn reject_having_non_empty() {
        let request = GetDocumentsRequestV1 {
            having: vec![0x01, 0x02],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(validate_and_route_for_tests(&request, &[]), "HAVING clause");
    }

    #[test]
    fn reject_group_by_with_documents() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Documents as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[]),
            "GROUP BY with SELECT DOCUMENTS",
        );
    }

    #[test]
    fn reject_group_by_field_not_in_where_clauses() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[]),
            "GROUP BY on field 'color' which is not constrained",
        );
    }

    #[test]
    fn reject_group_by_more_than_two_fields() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[]),
            "GROUP BY with more than two fields",
        );
    }

    #[test]
    fn reject_two_field_group_by_outside_compound_shape() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["color".to_string(), "brand".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ];
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &where_clauses),
            "two-field GROUP BY outside the `(In, range)` compound shape",
        );
    }

    #[test]
    fn accept_count_with_empty_group_by_routes_to_aggregate() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            ..empty_v1_request()
        };
        assert_eq!(
            validate_and_route_for_tests(&request, &[]).unwrap(),
            "count_aggregate"
        );
    }

    #[test]
    fn reject_count_aggregate_with_limit() {
        // Aggregate count is a single row; a `limit` is structurally
        // meaningless and previously caused Drive's per-In fan-out
        // to honor it and return a partial sum disguised as a total.
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            limit: Some(1),
            ..empty_v1_request()
        };
        let where_clauses = vec![WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: platform_value!([30u32, 40u32]),
        }];
        match validate_and_route_for_tests(&request, &where_clauses) {
            Err(QueryError::Query(QuerySyntaxError::InvalidLimit(msg))) => {
                assert!(
                    msg.contains("aggregate count is a single row"),
                    "expected aggregate-count limit-rejection message, got: {msg}"
                );
            }
            other => panic!("expected InvalidLimit, got {other:?}"),
        }
    }

    #[test]
    fn reject_count_group_by_in_with_limit() {
        // GROUP BY on an `In` field returns at most `|In|` entries
        // (capped at 100 by `WhereClause::in_values()`). A `limit`
        // is either redundant (≤ 100) or would silently truncate
        // the proof to fewer In branches than requested — the
        // PointLookupProof path can't represent a partial-In
        // selection in its `SizedQuery`, so the limit gets dropped
        // before reaching the path-query builder. Reject upstream
        // to make the contract explicit.
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["age".to_string()],
            limit: Some(1),
            ..empty_v1_request()
        };
        let where_clauses = vec![WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: platform_value!([30u32, 40u32, 50u32]),
        }];
        match validate_and_route_for_tests(&request, &where_clauses) {
            Err(QueryError::Query(QuerySyntaxError::InvalidLimit(msg))) => {
                assert!(
                    msg.contains("bounded by the In array"),
                    "expected GroupByIn limit-rejection message, got: {msg}"
                );
            }
            other => panic!("expected InvalidLimit, got {other:?}"),
        }
    }

    #[test]
    fn reject_single_field_group_by_on_in_field_when_range_also_constrained() {
        // `group_by=[in_field]` looks well-formed in isolation, but
        // the simultaneous range clause forces Drive's compound walk
        // to emit `(in_key, key)` rows that don't match the caller's
        // single-field grouping. Caller must spell out the compound
        // shape explicitly with `[in_field, range_field]`.
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["brand".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ];
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &where_clauses),
            "single-field GROUP BY when both `In` and range clauses are present",
        );
    }

    #[test]
    fn reject_single_field_group_by_on_range_field_when_in_also_constrained() {
        // Mirror of the above for the range-field branch: same
        // compound-shape mismatch, different `group_by` entry.
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ];
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &where_clauses),
            "single-field GROUP BY when both `In` and range clauses are present",
        );
    }

    #[test]
    fn accept_count_group_by_in_field_routes_to_in_entries() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["brand".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        }];
        assert_eq!(
            validate_and_route_for_tests(&request, &where_clauses).unwrap(),
            "count_entries_via_in_field"
        );
    }

    #[test]
    fn accept_count_group_by_range_field_routes_to_range_entries() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        }];
        assert_eq!(
            validate_and_route_for_tests(&request, &where_clauses).unwrap(),
            "count_entries_via_range_field"
        );
    }

    #[test]
    fn accept_count_group_by_compound_routes_to_compound_entries() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["brand".to_string(), "color".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ];
        assert_eq!(
            validate_and_route_for_tests(&request, &where_clauses).unwrap(),
            "count_entries_via_compound"
        );
    }

    #[test]
    fn e2e_documents_select_matches_v0() {
        use dpp::data_contract::DataContractFactory;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();
        store_data_contract(&platform, &contract, version);

        let document_type = contract.document_type_for_name("widget").expect("widget");
        for i in 1..=3u8 {
            let doc = document_type
                .random_document(Some(i as u64), platform_version)
                .expect("random doc");
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        // v0 baseline.
        let request_v0 = GetDocumentsRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: 0,
            prove: false,
            start: None,
        };
        let v0_result = platform
            .query_documents_v0(request_v0, &state, version)
            .expect("v0 query");
        let v0_docs = match v0_result.data {
            Some(r) => match r.result {
                Some(get_documents_response_v0::Result::Documents(d)) => d.documents,
                other => panic!("v0: expected Documents, got {:?}", other),
            },
            None => panic!("v0: empty data"),
        };
        assert_eq!(v0_docs.len(), 3);

        // v1 equivalent.
        let request_v1 = GetDocumentsRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: false,
            select: V1Select::Documents as i32,
            group_by: Vec::new(),
            having: Vec::new(),
        };
        let v1_result = platform
            .query_documents_v1(request_v1, &state, version)
            .expect("v1 query");
        let v1_docs = match v1_result.data {
            Some(r) => match r.result {
                Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Documents(d)),
                })) => d.documents,
                other => panic!("v1: expected Documents, got {:?}", other),
            },
            None => panic!("v1: empty data"),
        };
        assert_eq!(v1_docs, v0_docs, "v0 and v1 returned the same documents");
    }

    #[test]
    fn e2e_having_rejection_surfaces_in_response() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "anything".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: false,
            select: V1Select::Count as i32,
            group_by: Vec::new(),
            having: vec![0xFF, 0xFE],
        };
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(
            !result.errors.is_empty(),
            "expected validation error for HAVING request"
        );
        match &result.errors[0] {
            QueryError::Query(QuerySyntaxError::Unsupported(msg)) => {
                assert!(
                    msg.contains("HAVING") && msg.contains("not yet implemented"),
                    "expected HAVING-specific message, got: {}",
                    msg
                );
            }
            other => panic!("expected Unsupported error, got {:?}", other),
        }
    }

    #[test]
    fn reject_start_with_select_count() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: Some(V1Start::StartAfter(vec![1u8; 32])),
            prove: false,
            select: V1Select::Count as i32,
            group_by: Vec::new(),
            having: Vec::new(),
        };
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(!result.errors.is_empty(), "expected validation error");
        match &result.errors[0] {
            QueryError::Query(QuerySyntaxError::Unsupported(msg)) => {
                assert!(
                    msg.contains("start_after") && msg.contains("not yet implemented"),
                    "expected start_after-specific message, got: {}",
                    msg
                );
            }
            other => panic!("expected Unsupported error, got {:?}", other),
        }
    }
}

#[cfg(test)]
mod ported_v0_count_tests {
    //! Integration tests ported from the (now-removed)
    //! `document_count_query::v0` test module — exercises every count
    //! shape that the v0 endpoint exposed, now through the v1
    //! handler. Mechanical 1:1 translation: the request type changes
    //! from `GetDocumentsCountRequestV0` to `GetDocumentsRequestV1`
    //! with `select=COUNT` and the `return_distinct_counts_in_range`
    //! flag mapped to an explicit `group_by`; the response pattern
    //! changes from `GetDocumentsCountResponseV0`'s
    //! `Counts(CountResults { … })` envelope to v1's nested
    //! `Data(ResultData { variant: Counts(CountResults { … }) })`.
    //!
    //! Same fixtures + assertions as before — these tests are the
    //! load-bearing coverage for the entire count-execution surface
    //! and the port preserves them verbatim under the new wire shape.
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select as V1Select;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::DocumentV0Setters;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Builds an in-memory v12 contract with a `widget` document
    /// type that has `documentsCountable: true` — the type's
    /// primary-key tree becomes a CountTree, enabling the
    /// unfiltered total-count fast path on both no-proof and prove
    /// paths.
    fn build_documents_countable_widget_contract() -> dpp::prelude::DataContract {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "documentsCountable": true,
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned()
    }

    fn serialize_where_clauses_to_cbor(where_clauses: Vec<Value>) -> Vec<u8> {
        use ciborium::value::Value as CborValue;
        let cbor: CborValue = TryInto::<CborValue>::try_into(Value::Array(where_clauses))
            .expect("expected to convert where clauses to cbor value");
        let mut out = Vec::new();
        ciborium::ser::into_writer(&cbor, &mut out).expect("expected to serialize where clauses");
        out
    }

    fn store_person_document(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        data_contract: &dpp::prelude::DataContract,
        id: [u8; 32],
        first_name: &str,
        last_name: &str,
        age: u64,
        platform_version: &PlatformVersion,
    ) {
        use dpp::document::{Document, DocumentV0};
        use std::collections::BTreeMap;

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut properties = BTreeMap::new();
        properties.insert("firstName".to_string(), Value::Text(first_name.to_string()));
        properties.insert("lastName".to_string(), Value::Text(last_name.to_string()));
        properties.insert("age".to_string(), Value::U64(age));

        let document: Document = DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        store_document(
            platform,
            data_contract,
            document_type,
            &document,
            platform_version,
        );
    }

    /// Build a `SELECT COUNT` v1 request with the given knobs. Keeps
    /// each test's body focused on the per-test setup + assertion.
    #[allow(clippy::too_many_arguments)]
    fn count_v1_request(
        data_contract_id: Vec<u8>,
        document_type: &str,
        where_bytes: Vec<u8>,
        order_by_bytes: Vec<u8>,
        group_by: Vec<String>,
        limit: Option<u32>,
        prove: bool,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id,
            document_type: document_type.to_string(),
            r#where: where_bytes,
            order_by: order_by_bytes,
            limit,
            start: None,
            prove,
            select: V1Select::Count as i32,
            group_by,
            having: Vec::new(),
        }
    }

    /// Match the inner `Data(ResultData { variant: Counts(CountResults
    /// { variant: AggregateCount(_) }) })` shape and return the count.
    /// Panics on any other response shape.
    fn unwrap_aggregate(response: GetDocumentsResponseV1) -> u64 {
        match response.result {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant:
                    Some(result_data::Variant::Counts(CountResults {
                        variant: Some(count_results::Variant::AggregateCount(total)),
                    })),
            })) => total,
            other => panic!("expected aggregate count result, got {:?}", other),
        }
    }

    /// Match the inner `Data(ResultData { variant: Counts(CountResults
    /// { variant: Entries(_) }) })` shape and return the entries.
    fn unwrap_entries(response: GetDocumentsResponseV1) -> Vec<CountEntry> {
        match response.result {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant:
                    Some(result_data::Variant::Counts(CountResults {
                        variant: Some(count_results::Variant::Entries(entries)),
                    })),
            })) => entries.entries,
            other => panic!("expected per-key entries result, got {:?}", other),
        }
    }

    /// Unfiltered total count via the `documentsCountable: true`
    /// fast path. Ported from v0-count's `test_documents_count_no_prove`.
    #[test]
    fn ported_documents_count_no_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let contract = build_documents_countable_widget_contract();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for i in 1..=5u8 {
            let random_document = document_type
                .random_document(Some(i as u64), platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            vec![],
            Vec::new(),
            /* group_by = */ Vec::new(),
            /* limit = */ None,
            /* prove = */ false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(
            unwrap_aggregate(result.data.expect("data")),
            5,
            "expected count of 5 documents"
        );
    }

    /// Empty contract → aggregate 0. Ported from
    /// `test_documents_count_empty_result`.
    #[test]
    fn ported_documents_count_empty_result() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let contract = build_documents_countable_widget_contract();
        store_data_contract(&platform, &contract, version);

        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            vec![],
            Vec::new(),
            Vec::new(),
            None,
            false,
        );
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(
            unwrap_aggregate(result.data.expect("data")),
            0,
            "expected count of 0 documents"
        );
    }

    /// `In` clause + per-In entries. The v0-count endpoint did this
    /// implicitly (any In → PerInValue → entries); v1 makes the
    /// grouping explicit via `group_by=["age"]`. Ported from
    /// `test_documents_count_with_in_operator`.
    #[test]
    fn ported_documents_count_with_in_operator() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        for (id, name, age) in [
            ([1u8; 32], "Alice", 30u64),
            ([2u8; 32], "Bob", 30),
            ([3u8; 32], "Carol", 30),
            ([4u8; 32], "Dave", 40),
            ([5u8; 32], "Eve", 40),
            ([6u8; 32], "Frank", 50),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                name,
                "Smith",
                age,
                platform_version,
            );
        }

        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
            Vec::new(),
            vec!["age".to_string()],
            None,
            false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let entries = unwrap_entries(result.data.expect("data"));
        let total: u64 = entries.iter().map(|e| e.count).sum();
        assert_eq!(total, 5, "expected count of 5 (3 age=30 + 2 age=40)");
    }

    /// Range without a `range_countable` index → picker rejection.
    /// Ported from
    /// `test_documents_count_range_without_range_countable_index_returns_clear_error`.
    #[test]
    fn ported_range_without_range_countable_index_returns_clear_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text(">".to_string()),
            Value::U64(20),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
            Vec::new(),
            Vec::new(),
            None,
            false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to return validation error");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("range_countable")
            ) || matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(msg))]
                    if msg.contains("range_countable")
            ),
            "expected range_countable-index rejection, got {:?}",
            result.errors
        );
    }

    /// `prove = true` + Equal-on-single-property-countable-index →
    /// CountTree element proof. Ported from
    /// `test_documents_count_with_prove_and_covering_equal`.
    #[test]
    fn ported_documents_count_with_prove_and_covering_equal() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for first_name in ["Alice", "Alice", "Bob"] {
            let mut doc = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("firstName".to_string(), Value::Text(first_name.to_string()));
            props.insert("lastName".to_string(), Value::Text("Smith".to_string()));
            props.insert("age".to_string(), Value::U64(30));
            doc.set_properties(props);
            store_document(
                &platform,
                &data_contract,
                document_type,
                &doc,
                platform_version,
            );
        }

        let where_clauses = vec![Value::Array(vec![
            Value::Text("firstName".to_string()),
            Value::Text("==".to_string()),
            Value::Text("Alice".to_string()),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
            Vec::new(),
            Vec::new(),
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for covered prove count"
                );
            }
            other => panic!("expected Proof response, got {:?}", other),
        }
    }

    /// `prove = true` with no covering index → clear error. Ported
    /// from `test_documents_count_prove_without_covering_index_returns_clear_error`.
    #[test]
    fn ported_prove_without_covering_index_returns_clear_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            vec![],
            Vec::new(),
            Vec::new(),
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to surface a validation error");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(
                    QuerySyntaxError::WhereClauseOnNonIndexedProperty(msg),
                )] if msg.contains("countable")
            ),
            "expected covering-index rejection, got {:?}",
            result.errors
        );
    }

    /// `prove = true` + `In` → CountTree element proof. Ported
    /// from `test_documents_count_with_in_and_prove_returns_proof`.
    /// v1 expresses the per-In emission explicitly via
    /// `group_by=["age"]`; the underlying drive routing decision
    /// (PointLookupProof) and emitted proof bytes are the same as
    /// the v0-count test.
    #[test]
    fn ported_documents_count_with_in_and_prove_returns_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        for (id, name, age) in [
            ([1u8; 32], "Alice", 30u64),
            ([2u8; 32], "Bob", 30),
            ([3u8; 32], "Carol", 30),
            ([4u8; 32], "Dave", 40),
            ([5u8; 32], "Eve", 40),
            ([6u8; 32], "Frank", 50),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                name,
                "Smith",
                age,
                platform_version,
            );
        }

        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];
        let order_by = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("asc".to_string()),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
            serialize_where_clauses_to_cbor(order_by),
            vec!["age".to_string()],
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for In + prove count"
                );
            }
            other => panic!(
                "expected Proof response from In + prove count, got {:?}",
                other
            ),
        }
    }

    /// Range count happy path — sum + distinct + limit + direction.
    /// Ported from `test_documents_count_range_query_no_prove`. v1
    /// translates `return_distinct_counts_in_range=true` to
    /// `group_by=["color"]` and the summed mode keeps `group_by=[]`.
    #[test]
    fn ported_documents_count_range_query_no_prove() {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for (i, color) in ["red", "red", "blue", "green", "green", "green"]
            .iter()
            .enumerate()
        {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), platform_version)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        let make_request = |group_by: Vec<String>, limit: Option<u32>, ascending: Option<bool>| {
            let where_clauses = vec![Value::Array(vec![
                Value::Text("color".to_string()),
                Value::Text(">".to_string()),
                Value::Text("blue".to_string()),
            ])];
            let order_by_bytes = match ascending {
                Some(asc) => serialize_where_clauses_to_cbor(vec![Value::Array(vec![
                    Value::Text("color".to_string()),
                    Value::Text(if asc { "asc" } else { "desc" }.to_string()),
                ])]),
                None => Vec::new(),
            };
            count_v1_request(
                contract.id().to_vec(),
                "widget",
                serialize_where_clauses_to_cbor(where_clauses),
                order_by_bytes,
                group_by,
                limit,
                false,
            )
        };

        // Sum mode: green(3) + red(2) = 5.
        let result = platform
            .query_documents_v1(make_request(Vec::new(), None, None), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(unwrap_aggregate(result.data.expect("data")), 5);

        // Distinct mode ascending: [(green, 3), (red, 2)].
        let result = platform
            .query_documents_v1(
                make_request(vec!["color".to_string()], None, Some(true)),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let entries = unwrap_entries(result.data.expect("data"));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, b"green".to_vec());
        assert_eq!(entries[0].count, 3);
        assert_eq!(entries[1].key, b"red".to_vec());
        assert_eq!(entries[1].count, 2);

        // Distinct with limit=1.
        let result = platform
            .query_documents_v1(
                make_request(vec!["color".to_string()], Some(1), Some(true)),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        let entries = unwrap_entries(result.data.expect("data"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, b"green".to_vec());

        // Distinct descending: [(red, 2), (green, 3)].
        let result = platform
            .query_documents_v1(
                make_request(vec!["color".to_string()], None, Some(false)),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        let entries = unwrap_entries(result.data.expect("data"));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, b"red".to_vec());
        assert_eq!(entries[1].key, b"green".to_vec());
    }

    /// `RangeDistinctProof` dispatch — `group_by=["color"]` +
    /// `prove=true` + range clause. Ported from
    /// `test_documents_count_range_with_prove_and_distinct_returns_proof`.
    #[test]
    fn ported_documents_count_range_with_prove_and_distinct_returns_proof() {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let platform_version = PlatformVersion::latest();
        for (i, color) in ["red", "red", "green", "green", "green", "blue"]
            .iter()
            .enumerate()
        {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), platform_version)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        let where_clauses = vec![Value::Array(vec![
            Value::Text("color".to_string()),
            Value::Text(">".to_string()),
            Value::Text("blue".to_string()),
        ])];
        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            serialize_where_clauses_to_cbor(where_clauses),
            Vec::new(),
            vec!["color".to_string()],
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query should succeed");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for non-empty range result"
                );
            }
            other => panic!("expected Proof response, got {:?}", other),
        }
    }
}
