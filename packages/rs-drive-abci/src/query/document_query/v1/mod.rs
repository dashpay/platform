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
    DocumentCountRequest, DocumentCountResponse, SplitCountEntry, WhereClause, WhereOperator,
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
                QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
            ));
        }
    };
    let mut clauses = Vec::with_capacity(array.len());
    for entry in array {
        let components = match entry {
            Value::Array(c) => c,
            _ => {
                return Err(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
                ));
            }
        };
        let clause = WhereClause::from_components(&components).map_err(|e| {
            QueryError::Query(QuerySyntaxError::InvalidFormatWhereClause(Box::leak(
                format!("invalid where clause components: {e}").into_boxed_str(),
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

            match request_v1.group_by.as_slice() {
                [] => Ok(RoutingDecision::CountAggregate),
                [field] => {
                    if Some(field.as_str()) == in_field {
                        Ok(RoutingDecision::CountEntriesViaInField)
                    } else if Some(field.as_str()) == range_field {
                        Ok(RoutingDecision::CountEntriesViaRangeField)
                    } else {
                        Err(not_yet_implemented(&format!(
                            "GROUP BY on field '{}' which is not constrained by an \
                             `In` or range where clause",
                            field
                        )))
                    }
                }
                [first, second] => {
                    if Some(first.as_str()) == in_field && Some(second.as_str()) == range_field {
                        Ok(RoutingDecision::CountEntriesViaCompound)
                    } else {
                        Err(not_yet_implemented(
                            "two-field GROUP BY outside the `(In, range)` compound \
                             shape (the existing compound count path orders entries \
                             as `(in_key, key)`; other orderings would need a new \
                             merk walk)",
                        ))
                    }
                }
                _ => Err(not_yet_implemented("GROUP BY with more than two fields")),
            }
        }
    }
}

/// Outcome of `validate_and_route` — names the executor path the
/// v1 request will dispatch to.
enum RoutingDecision {
    Documents,
    CountAggregate,
    CountEntriesViaInField,
    CountEntriesViaRangeField,
    CountEntriesViaCompound,
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
        RoutingDecision::CountAggregate => "count_aggregate",
        RoutingDecision::CountEntriesViaInField => "count_entries_via_in_field",
        RoutingDecision::CountEntriesViaRangeField => "count_entries_via_range_field",
        RoutingDecision::CountEntriesViaCompound => "count_entries_via_compound",
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
            RoutingDecision::CountAggregate => self.dispatch_count_v1(
                request_v1,
                /* return_distinct_counts_in_range = */ false,
                /* expect_aggregate = */ true,
                platform_state,
                platform_version,
            ),
            RoutingDecision::CountEntriesViaInField => self.dispatch_count_v1(
                request_v1,
                /* return_distinct_counts_in_range = */ false,
                /* expect_aggregate = */ false,
                platform_state,
                platform_version,
            ),
            RoutingDecision::CountEntriesViaRangeField
            | RoutingDecision::CountEntriesViaCompound => self.dispatch_count_v1(
                request_v1,
                /* return_distinct_counts_in_range = */ true,
                /* expect_aggregate = */ false,
                platform_state,
                platform_version,
            ),
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

    /// Forward a `select = COUNT` request through drive's count
    /// dispatcher directly. Replaces the old delegation through the
    /// v0-count abci handler (which has been removed in this PR);
    /// the wire response is now `GetDocumentsResponseV1` with
    /// the inner `ResultData.counts` variant for non-proof results.
    fn dispatch_count_v1(
        &self,
        request_v1: GetDocumentsRequestV1,
        return_distinct_counts_in_range: bool,
        expect_aggregate: bool,
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
            return_distinct_counts_in_range,
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
                if expect_aggregate {
                    // `select=COUNT, group_by=[]` against a request
                    // that drove a PerInValue execution (In + no
                    // range + no prove). Sum entries into a single
                    // aggregate before emission. `saturating_add`
                    // on the off-chance an operator-misconfigured
                    // count tree exceeds u64; realistic ceiling is
                    // `|In| × max_per-branch-count`, well under u64.
                    let total: u64 = entries
                        .iter()
                        .map(|e| e.count)
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
        count: e.count,
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
