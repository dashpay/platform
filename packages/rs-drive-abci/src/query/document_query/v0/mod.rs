use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV0;
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, GetDocumentsResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{DriveDocumentQuery, OrderClause, WhereClause};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_documents_v0(
        &self,
        GetDocumentsRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            order_by,
            limit,
            prove,
            start,
        }: GetDocumentsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV0>, Error> {
        // CBOR-decode the v0 wire fields into `Value` shells. The
        // typed-clauses path picks up after this and is shared with
        // the v1 handler — see `query_documents_typed` below.
        let where_value = if r#where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(r#where.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'where' query from cbor".to_string(),
                    ))
                }))
        };

        let order_by_value: Option<Value> = if !order_by.is_empty() {
            check_validation_result_with_data!(ciborium::de::from_reader(order_by.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'order_by' query from cbor".to_string(),
                    ))
                }))
        } else {
            None
        };

        // Parse the decoded `Value` shells into structured clauses.
        // `DriveDocumentQuery::from_decomposed_values` historically
        // did this internally; lifting the parse into the abci layer
        // lets v0 and v1 (whose wire is already typed) share the
        // same execution helper without re-encoding bytes.
        let where_clauses: Vec<WhereClause> = match where_value {
            Value::Null => Vec::new(),
            Value::Array(clauses) => {
                let parsed: Result<Vec<WhereClause>, _> = clauses
                    .iter()
                    .map(|wc| match wc {
                        Value::Array(components) => WhereClause::from_components(components),
                        _ => Err(drive::error::Error::Query(
                            QuerySyntaxError::InvalidFormatWhereClause(
                                "where clause must be an array".to_string(),
                            ),
                        )),
                    })
                    .collect();
                check_validation_result_with_data!(parsed.map_err(|e| QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause(format!(
                        "invalid where clause components: {e}"
                    ))
                )))
            }
            _ => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause(
                        "where clause must be an array".to_string(),
                    ),
                )));
            }
        };

        let order_by_clauses: Vec<OrderClause> = match order_by_value {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(clauses)) => {
                let parsed: Result<Vec<OrderClause>, _> = clauses
                    .iter()
                    .map(|oc| match oc {
                        Value::Array(components) => OrderClause::from_components(components)
                            .map_err(|_| {
                                QueryError::Query(QuerySyntaxError::InvalidOrderByProperties(
                                    "invalid order_by clause components",
                                ))
                            }),
                        _ => Err(QueryError::Query(
                            QuerySyntaxError::InvalidOrderByProperties(
                                "order_by clause must be an array",
                            ),
                        )),
                    })
                    .collect();
                check_validation_result_with_data!(parsed)
            }
            _ => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidOrderByProperties("order_by must be an array"),
                )));
            }
        };

        self.query_documents_typed(
            data_contract_id,
            document_type_name,
            where_clauses,
            order_by_clauses,
            // v0 wire's `uint32` limit: `0` is the sentinel for
            // "use server default"; `> u16::MAX` is rejected.
            Some(limit),
            prove,
            start,
            platform_state,
            platform_version,
        )
    }

    /// Shared execution pipeline for `getDocuments` — consumes
    /// already-structured `where_clauses` / `order_by_clauses` and
    /// reuses the same drive `DriveDocumentQuery` build + execute
    /// path under both v0 (CBOR-decoded into typed) and v1 (proto-
    /// converted into typed) wire envelopes.
    ///
    /// `limit_u32` semantics mirror the v0 wire field:
    /// - `None` (v1's `optional uint32 = None`) → use the server
    ///   default (`drive_config.default_query_limit`).
    /// - `Some(0)` (v0's "unset" sentinel) → same as `None`.
    /// - `Some(N > 0)` → explicit cap; rejected if `N > u16::MAX`.
    ///
    /// v1 callers map their `Option<u32>` directly (None → None,
    /// Some(0) is pre-rejected upstream by `validate_and_route` so
    /// can't reach this helper).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn query_documents_typed(
        &self,
        data_contract_id: Vec<u8>,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        order_by_clauses: Vec<OrderClause>,
        limit_u32: Option<u32>,
        prove: bool,
        start: Option<Start>,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV0>, Error> {
        let contract_id: Identifier = check_validation_result_with_data!(data_contract_id
            .try_into()
            .map_err(|_| QueryError::InvalidArgument(
                "id must be a valid identifier (32 bytes long)".to_string()
            )));

        let (_, contract) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;

        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            )
        )));

        let contract_ref = &contract.contract;

        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let (start_at_included, start_at) = if let Some(start) = start {
            match start {
                Start::StartAfter(after) => (
                    false,
                    Some(check_validation_result_with_data!(after
                        .try_into()
                        .map_err(|_| QueryError::Query(
                            QuerySyntaxError::InvalidStartsWithClause(
                                "start after should be a 32 byte identifier",
                            )
                        )))),
                ),
                Start::StartAt(at) => (
                    true,
                    Some(check_validation_result_with_data!(at.try_into().map_err(
                        |_| QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                            "start at should be a 32 byte identifier",
                        ))
                    ))),
                ),
            }
        } else {
            (true, None)
        };

        // Translate the wire-level `Option<u32>` to the `Option<u16>`
        // `DriveDocumentQuery::from_typed_clauses` expects. Both
        // `None` and `Some(0)` map to `Some(default_query_limit)`
        // (server default applies); values exceeding `u16::MAX` are
        // rejected here so the cast below is safe.
        let limit_u16 = match limit_u32 {
            Some(n) if n > u16::MAX as u32 => {
                return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                    QuerySyntaxError::InvalidLimit(format!("limit {} out of bounds", n)),
                )));
            }
            None | Some(0) => Some(self.config.drive.default_query_limit),
            Some(n) => Some(n as u16),
        };

        let drive_query =
            check_validation_result_with_data!(DriveDocumentQuery::from_typed_clauses(
                where_clauses,
                order_by_clauses,
                limit_u16,
                start_at,
                start_at_included,
                None,
                contract_ref,
                document_type,
                &self.config.drive,
                platform_version,
            ));

        let response = if prove {
            let proof =
                match drive_query.execute_with_proof(&self.drive, None, None, platform_version) {
                    Ok(result) => result.0,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let results = match drive_query.execute_raw_results_no_proof(
                &self.drive,
                None,
                None,
                platform_version,
            ) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Documents(
                    get_documents_response_v0::Documents { documents: results },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{
        assert_invalid_identifier, setup_platform, store_data_contract, store_document,
    };
    use ciborium::value::Value as CborValue;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::query::{InternalClauses, OrderClause, WhereClause, WhereOperator};
    use indexmap::IndexMap;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    fn test_invalid_document_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDocumentsRequestV0 {
            data_contract_id: vec![0; 8],
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_data_contract_not_found_in_documents_request() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let data_contract_id = vec![0; 32];

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.clone(),
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(msg))] if msg == &"contract not found when querying from value with contract info"
        ));
    }

    #[test]
    fn test_absent_document_type() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "fakeDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains(format!(
            "document type {} not found for contract",
            document_type,
        ).as_str())))
    }

    #[test]
    fn test_invalid_where_clause() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![0x9F], // Incomplete CBOR array
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DeserializationError(msg))] if msg == "unable to decode 'where' query from cbor"
        ))
    }

    #[test]
    fn test_invalid_order_by_clause() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![0x9F], // Incomplete CBOR array
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DeserializationError(msg))] if msg == "unable to decode 'order_by' query from cbor"
        ));
    }

    #[test]
    fn test_invalid_start_at_clause() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: Some(Start::StartAt(vec![0; 8])),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg))] if msg == &"start at should be a 32 byte identifier"
        ))
    }

    #[test]
    fn test_invalid_start_after_clause() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: Some(Start::StartAfter(vec![0; 8])),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg))] if msg == &"start after should be a 32 byte identifier"
        ));
    }

    #[test]
    fn test_invalid_limit() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";
        let limit = u32::MAX;

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(msg))] if msg == &format!("limit {} out of bounds", limit)
        ))
    }

    #[test]
    fn test_documents_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Documents(documents)),
                metadata: Some(_),
            }) if documents.documents.is_empty()
        ));
    }

    #[test]
    fn test_documents_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type = "niceDocument";

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type.to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_documents_single_item_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type_name = "niceDocument";
        let document_type = created_data_contract
            .data_contract()
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let random_document = document_type
            .random_document(Some(4), platform_version)
            .expect("expected to get random document");

        store_document(
            &platform,
            created_data_contract.data_contract(),
            document_type,
            &random_document,
            platform_version,
        );

        let drive_document_query = DriveDocumentQuery {
            contract: created_data_contract.data_contract(),
            document_type,
            internal_clauses: Default::default(),
            offset: None,
            limit: Some(1),
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            limit: 1,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(documents.len(), 1);
        assert_eq!(documents.first().expect("first"), &random_document);
    }

    #[test]
    fn test_documents_range_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type_name = "niceDocument";
        let document_type = created_data_contract
            .data_contract()
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_id = BTreeMap::new();
        for _i in 0..20 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                created_data_contract.data_contract(),
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_id.insert(random_document.id(), random_document);
        }

        let drive_document_query = DriveDocumentQuery {
            contract: created_data_contract.data_contract(),
            document_type,
            internal_clauses: Default::default(),
            offset: None,
            limit: Some(10),
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            limit: 10,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 10);
        assert_eq!(
            queried_documents.get(9).expect("first"),
            documents_by_id
                .values()
                .nth(9)
                .expect("expected to get 9th document")
        );
    }

    #[test]
    fn test_documents_start_after_proof_primary_index() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type_name = "niceDocument";
        let document_type = created_data_contract
            .data_contract()
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_id = BTreeMap::new();
        for _i in 0..20 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                created_data_contract.data_contract(),
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_id.insert(random_document.id(), random_document);
        }

        let after = documents_by_id
            .keys()
            .nth(9)
            .expect("expected to get 9th document")
            .to_buffer();

        let drive_document_query = DriveDocumentQuery {
            contract: created_data_contract.data_contract(),
            document_type,
            internal_clauses: Default::default(),
            offset: None,
            limit: Some(10),
            order_by: Default::default(),
            start_at: Some(after),
            start_at_included: false,
            block_time_ms: None,
        };

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            limit: 10,
            order_by: vec![],
            prove: true,
            start: Some(Start::StartAfter(after.to_vec())),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 10);
        assert_eq!(
            queried_documents.get(9).expect("last"),
            documents_by_id
                .values()
                .nth(19)
                .expect("expected to get 9th document")
        );
    }

    fn serialize_vec_to_cbor<T: Into<Value>>(input: Vec<T>) -> Result<Vec<u8>, Error> {
        let values = Value::Array(
            input
                .into_iter()
                .map(|v| v.into() as Value)
                .collect::<Vec<Value>>(),
        );

        let cbor_values: CborValue = TryInto::<CborValue>::try_into(values)
            .map_err(|e| Error::Protocol(dpp::ProtocolError::EncodingError(e.to_string())))?;

        let mut serialized = Vec::new();
        ciborium::ser::into_writer(&cbor_values, &mut serialized)
            .map_err(|e| Error::Protocol(dpp::ProtocolError::EncodingError(e.to_string())))?;

        Ok(serialized)
    }

    #[test]
    fn test_documents_start_after_proof_secondary_index() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let withdrawals = platform
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)
            .expect("expected the withdrawals system contract");

        let data_contract_id = withdrawals.id();
        let document_type_name = "withdrawal";
        let document_type = withdrawals
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_created_at = BTreeMap::new();

        // Define the base time as the current system time
        let base_time = 1730028481000;

        for i in 0..20 {
            let created_at = base_time + i * 20000;
            // Create a Document with the desired properties
            let random_document: Document = DocumentV0 {
                id: Identifier::random_with_rng(&mut std_rng),
                owner_id: Identifier::random_with_rng(&mut std_rng),
                properties: {
                    let mut properties = BTreeMap::new();
                    properties.insert("status".to_string(), Value::I64(0)); // Always queued
                    properties.insert("pooling".to_string(), Value::I64(0)); // Always 0
                    properties.insert("coreFeePerByte".to_string(), Value::I64(1)); // Always 1
                    properties.insert("amount".to_string(), Value::I64(1000)); // Set a minimum amount of 1000
                    properties.insert("outputScript".to_string(), Value::Bytes(vec![])); // Set an empty output script
                    properties
                },
                revision: Some(1),            // Example revision
                created_at: Some(created_at), // Set created_at
                updated_at: Some(created_at), // Set updated_at
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
                &platform,
                &withdrawals,
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_created_at.insert(created_at, random_document);
        }

        let after = documents_by_created_at
            .values()
            .nth(9)
            .expect("expected to get 9th document")
            .id();

        let drive_document_query = DriveDocumentQuery {
            contract: &withdrawals,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: BTreeMap::from([
                    (
                        "status".to_string(),
                        WhereClause {
                            field: "status".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U8(0),
                        },
                    ),
                    (
                        "pooling".to_string(),
                        WhereClause {
                            field: "pooling".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U8(0),
                        },
                    ),
                    (
                        "coreFeePerByte".to_string(),
                        WhereClause {
                            field: "coreFeePerByte".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U32(1),
                        },
                    ),
                ]),
            },
            offset: None,
            limit: Some(10),
            order_by: IndexMap::from([(
                "$updatedAt".to_string(),
                OrderClause {
                    field: "$updatedAt".to_string(),
                    ascending: true,
                },
            )]),
            start_at: Some(after.to_buffer()),
            start_at_included: false,
            block_time_ms: None,
        };

        let where_clauses = serialize_vec_to_cbor(
            drive_document_query
                .internal_clauses
                .equal_clauses
                .values()
                .cloned()
                .collect(),
        )
        .expect("where clauses serialization should never fail");
        let order_by =
            serialize_vec_to_cbor(drive_document_query.order_by.values().cloned().collect())
                .expect("order by clauses serialization should never fail");

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: where_clauses,
            limit: 10,
            order_by,
            prove: true,
            start: Some(Start::StartAfter(after.to_vec())),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 10);
        assert_eq!(
            queried_documents.get(9).expect("last"),
            documents_by_created_at
                .values()
                .nth(19)
                .expect("expected to get 9th document")
        );
    }

    #[test]
    fn test_documents_start_after_proof_secondary_index_many_statuses() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let withdrawals = platform
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)
            .expect("expected the withdrawals system contract");

        let data_contract_id = withdrawals.id();
        let document_type_name = "withdrawal";
        let document_type = withdrawals
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_created_at = BTreeMap::new();

        // Define the base time as the current system time
        let base_time = 1730028481000;

        for i in 0..20 {
            let created_at = base_time + i * 20000;
            // Create a Document with the desired properties
            let random_document: Document = DocumentV0 {
                id: Identifier::random_with_rng(&mut std_rng),
                owner_id: Identifier::random_with_rng(&mut std_rng),
                properties: {
                    let mut properties = BTreeMap::new();
                    properties.insert("status".to_string(), Value::I64(i as i64 % 4)); // Always queued
                    properties.insert("pooling".to_string(), Value::I64(0)); // Always 0
                    properties.insert("coreFeePerByte".to_string(), Value::I64(1)); // Always 1
                    properties.insert("amount".to_string(), Value::I64(1000)); // Set a minimum amount of 1000
                    properties.insert("outputScript".to_string(), Value::Bytes(vec![])); // Set an empty output script
                    properties
                },
                revision: Some(1),            // Example revision
                created_at: Some(created_at), // Set created_at
                updated_at: Some(created_at), // Set updated_at
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
                &platform,
                &withdrawals,
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_created_at.insert(created_at, random_document);
        }

        let after = documents_by_created_at
            .values()
            .nth(9)
            .expect("expected to get 9th document")
            .id();

        let drive_document_query = DriveDocumentQuery {
            contract: &withdrawals,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: BTreeMap::from([
                    (
                        "status".to_string(),
                        WhereClause {
                            field: "status".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U8(0),
                        },
                    ),
                    (
                        "pooling".to_string(),
                        WhereClause {
                            field: "pooling".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U8(0),
                        },
                    ),
                    (
                        "coreFeePerByte".to_string(),
                        WhereClause {
                            field: "coreFeePerByte".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::U32(1),
                        },
                    ),
                ]),
            },
            offset: None,
            limit: Some(3),
            order_by: IndexMap::from([(
                "$updatedAt".to_string(),
                OrderClause {
                    field: "$updatedAt".to_string(),
                    ascending: true,
                },
            )]),
            start_at: Some(after.to_buffer()),
            start_at_included: false,
            block_time_ms: None,
        };

        let where_clauses = serialize_vec_to_cbor(
            drive_document_query
                .internal_clauses
                .equal_clauses
                .values()
                .cloned()
                .collect(),
        )
        .expect("where clauses serialization should never fail");
        let order_by =
            serialize_vec_to_cbor(drive_document_query.order_by.values().cloned().collect())
                .expect("order by clauses serialization should never fail");

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: where_clauses,
            limit: 3,
            order_by,
            prove: true,
            start: Some(Start::StartAfter(after.to_vec())),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 2);
        assert_eq!(
            queried_documents.get(1).expect("last"),
            documents_by_created_at
                .values()
                .nth(16)
                .expect("expected to get 2nd document")
        );
    }

    #[test]
    fn test_documents_proof_secondary_index_in_query() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let withdrawals = platform
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)
            .expect("expected the withdrawals system contract");

        let data_contract_id = withdrawals.id();
        let document_type_name = "withdrawal";
        let document_type = withdrawals
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_id = BTreeMap::new();

        // Define the base time as the current system time
        let base_time = 1730028481000;

        for i in 0..20 {
            let created_at = base_time + i * 20000;
            // Create a Document with the desired properties
            let random_document: Document = DocumentV0 {
                id: Identifier::random_with_rng(&mut std_rng),
                owner_id: Identifier::random_with_rng(&mut std_rng),
                properties: {
                    let mut properties = BTreeMap::new();
                    properties.insert("status".to_string(), Value::I64(i as i64 % 4)); // Always queued
                    properties.insert("pooling".to_string(), Value::I64(0)); // Always 0
                    properties.insert("coreFeePerByte".to_string(), Value::I64(1)); // Always 1
                    properties.insert("amount".to_string(), Value::I64(1000)); // Set a minimum amount of 1000
                    properties.insert("outputScript".to_string(), Value::Bytes(vec![])); // Set an empty output script
                    properties
                },
                revision: Some(1),            // Example revision
                created_at: Some(created_at), // Set created_at
                updated_at: Some(created_at), // Set updated_at
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
                &platform,
                &withdrawals,
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_id.insert(random_document.id(), random_document);
        }

        let drive_document_query = DriveDocumentQuery {
            contract: &withdrawals,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: vec![WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::U8(0),
                        Value::U8(1),
                        Value::U8(2),
                        Value::U8(3),
                        Value::U8(4),
                    ]),
                }],
                range_clause: None,
                equal_clauses: BTreeMap::default(),
            },
            offset: None,
            limit: Some(3),
            order_by: IndexMap::from([
                (
                    "status".to_string(),
                    OrderClause {
                        field: "status".to_string(),
                        ascending: true,
                    },
                ),
                (
                    "transactionIndex".to_string(),
                    OrderClause {
                        field: "transactionIndex".to_string(),
                        ascending: true,
                    },
                ),
            ]),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let mut where_clauses: Vec<_> = drive_document_query
            .internal_clauses
            .equal_clauses
            .values()
            .cloned()
            .collect();

        where_clauses.insert(
            0,
            drive_document_query
                .internal_clauses
                .in_clauses
                .first()
                .cloned()
                .unwrap(),
        );

        let where_clauses_serialized = serialize_vec_to_cbor(where_clauses)
            .expect("where clauses serialization should never fail");
        let order_by =
            serialize_vec_to_cbor(drive_document_query.order_by.values().cloned().collect())
                .expect("order by clauses serialization should never fail");

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: where_clauses_serialized,
            limit: 3,
            order_by,
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors are {:?}", result.errors);

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 3);
    }

    #[test]
    fn test_documents_start_after_proof_secondary_index_in_query() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let withdrawals = platform
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)
            .expect("expected the withdrawals system contract");

        let data_contract_id = withdrawals.id();
        let document_type_name = "withdrawal";
        let document_type = withdrawals
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_created_at = BTreeMap::new();

        // Define the base time as the current system time
        let base_time = 1730028481000;

        for i in 0..20 {
            let created_at = base_time + i * 20000;
            // Create a Document with the desired properties
            let random_document: Document = DocumentV0 {
                id: Identifier::random_with_rng(&mut std_rng),
                owner_id: Identifier::random_with_rng(&mut std_rng),
                properties: {
                    let mut properties = BTreeMap::new();
                    properties.insert("status".to_string(), Value::I64(i as i64 % 4)); // Always queued
                    properties.insert("pooling".to_string(), Value::I64(0)); // Always 0
                    properties.insert("coreFeePerByte".to_string(), Value::I64(1)); // Always 1
                    properties.insert("amount".to_string(), Value::I64(1000)); // Set a minimum amount of 1000
                    properties.insert("outputScript".to_string(), Value::Bytes(vec![])); // Set an empty output script
                    properties
                },
                revision: Some(1),            // Example revision
                created_at: Some(created_at), // Set created_at
                updated_at: Some(created_at), // Set updated_at
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
                &platform,
                &withdrawals,
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_created_at.insert(created_at, random_document);
        }

        let after = documents_by_created_at
            .values()
            .nth(4)
            .expect("expected to get 9th document")
            .id();

        let drive_document_query = DriveDocumentQuery {
            contract: &withdrawals,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: vec![WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::I64(0),
                        Value::I64(1),
                        Value::I64(2),
                        Value::I64(3),
                        Value::I64(4),
                    ]),
                }],
                range_clause: None,
                equal_clauses: BTreeMap::default(),
            },
            offset: None,
            limit: Some(3),
            order_by: IndexMap::from([
                (
                    "status".to_string(),
                    OrderClause {
                        field: "status".to_string(),
                        ascending: true,
                    },
                ),
                (
                    "transactionIndex".to_string(),
                    OrderClause {
                        field: "transactionIndex".to_string(),
                        ascending: true,
                    },
                ),
            ]),
            start_at: Some(after.to_buffer()),
            start_at_included: false,
            block_time_ms: None,
        };

        let mut where_clauses: Vec<_> = drive_document_query
            .internal_clauses
            .equal_clauses
            .values()
            .cloned()
            .collect();

        where_clauses.insert(
            0,
            drive_document_query
                .internal_clauses
                .in_clauses
                .first()
                .cloned()
                .unwrap(),
        );

        let where_clauses_serialized = serialize_vec_to_cbor(where_clauses)
            .expect("where clauses serialization should never fail");
        let order_by =
            serialize_vec_to_cbor(drive_document_query.order_by.values().cloned().collect())
                .expect("order by clauses serialization should never fail");

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: where_clauses_serialized,
            limit: 3,
            order_by,
            prove: true,
            start: Some(Start::StartAfter(after.to_vec())),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors are {:?}", result.errors);

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 3);
        assert_eq!(
            queried_documents.get(1).expect("last"),
            documents_by_created_at
                .values()
                .nth(16)
                .expect("expected to get 2nd document")
        );
    }

    //todo: this should be possible
    #[test]
    #[ignore]
    fn test_documents_start_after_proof_secondary_index_in_query_2() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let withdrawals = platform
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)
            .expect("expected the withdrawals system contract");

        let data_contract_id = withdrawals.id();
        let document_type_name = "withdrawal";
        let document_type = withdrawals
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(393);
        let mut documents_by_created_at = BTreeMap::new();

        // Define the base time as the current system time
        let base_time = 1730028481000;

        for i in 0..20 {
            let created_at = base_time + i * 20000;
            // Create a Document with the desired properties
            let random_document: Document = DocumentV0 {
                id: Identifier::random_with_rng(&mut std_rng),
                owner_id: Identifier::random_with_rng(&mut std_rng),
                properties: {
                    let mut properties = BTreeMap::new();
                    properties.insert("status".to_string(), Value::I64(i as i64 % 4)); // Always queued
                    properties.insert("pooling".to_string(), Value::I64(0)); // Always 0
                    properties.insert("coreFeePerByte".to_string(), Value::I64(1)); // Always 1
                    properties.insert("amount".to_string(), Value::I64(1000)); // Set a minimum amount of 1000
                    properties.insert("outputScript".to_string(), Value::Bytes(vec![])); // Set an empty output script
                    properties
                },
                revision: Some(1),            // Example revision
                created_at: Some(created_at), // Set created_at
                updated_at: Some(created_at), // Set updated_at
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
                &platform,
                &withdrawals,
                document_type,
                &random_document,
                platform_version,
            );
            documents_by_created_at.insert(created_at, random_document);
        }

        let after = documents_by_created_at
            .values()
            .nth(9)
            .expect("expected to get 9th document")
            .id();

        let drive_document_query = DriveDocumentQuery {
            contract: &withdrawals,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: vec![WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::I64(0),
                        Value::I64(1),
                        Value::I64(2),
                        Value::I64(3),
                        Value::I64(4),
                    ]),
                }],
                range_clause: None,
                equal_clauses: BTreeMap::from([
                    (
                        "pooling".to_string(),
                        WhereClause {
                            field: "pooling".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::I64(0),
                        },
                    ),
                    (
                        "coreFeePerByte".to_string(),
                        WhereClause {
                            field: "coreFeePerByte".to_string(),
                            operator: WhereOperator::Equal,
                            value: Value::I64(1),
                        },
                    ),
                ]),
            },
            offset: None,
            limit: Some(3),
            order_by: IndexMap::from([(
                "$updatedAt".to_string(),
                OrderClause {
                    field: "$updatedAt".to_string(),
                    ascending: true,
                },
            )]),
            start_at: Some(after.to_buffer()),
            start_at_included: false,
            block_time_ms: None,
        };

        let mut where_clauses: Vec<_> = drive_document_query
            .internal_clauses
            .equal_clauses
            .values()
            .cloned()
            .collect();

        where_clauses.insert(
            0,
            drive_document_query
                .internal_clauses
                .in_clauses
                .first()
                .cloned()
                .unwrap(),
        );

        let where_clauses_serialized = serialize_vec_to_cbor(where_clauses)
            .expect("where clauses serialization should never fail");
        let order_by =
            serialize_vec_to_cbor(drive_document_query.order_by.values().cloned().collect())
                .expect("order by clauses serialization should never fail");

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: where_clauses_serialized,
            limit: 3,
            order_by,
            prove: true,
            start: Some(Start::StartAfter(after.to_vec())),
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors are {:?}", result.errors);

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected proof")
        };

        let (_, queried_documents) = drive_document_query
            .verify_proof(&proof.grovedb_proof, platform_version)
            .expect("expected to verify proof");

        assert_eq!(queried_documents.len(), 2);
        assert_eq!(
            queried_documents.get(1).expect("last"),
            documents_by_created_at
                .values()
                .nth(16)
                .expect("expected to get 2nd document")
        );
    }

    /// When `prove: true` is set but the contract cannot be found, we must still
    /// return a query-error validation result (not an Err). This pins the early-return
    /// validation-ordering: contract lookup happens before prove-vs-no-prove branching.
    #[test]
    fn test_data_contract_not_found_with_prove_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let data_contract_id = vec![7u8; 32];

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.clone(),
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: true, // proof path
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(_))]
        ));
    }

    /// Invalid identifier must error out before the prove branch is reached.
    #[test]
    fn test_invalid_document_id_with_prove_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetDocumentsRequestV0 {
            data_contract_id: vec![0; 7], // wrong length
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    /// Invalid where clause must short-circuit even when proof is requested.
    /// This exercises the where-clause CBOR decoder error branch ahead of
    /// any prove-mode logic.
    #[test]
    fn test_invalid_where_clause_with_prove_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let request = GetDocumentsRequestV0 {
            data_contract_id: created_data_contract.data_contract().id().to_vec(),
            document_type: "niceDocument".to_string(),
            r#where: vec![0x9F], // malformed CBOR
            limit: 0,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DeserializationError(msg))]
                if msg == "unable to decode 'where' query from cbor"
        ));
    }

    /// When the limit is exactly u16::MAX + 1 (one past the bound) the InvalidLimit
    /// error path fires. This pins the boundary.
    #[test]
    fn test_limit_just_over_bound_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let limit = u16::MAX as u32 + 1;

        let request = GetDocumentsRequestV0 {
            data_contract_id: created_data_contract.data_contract().id().to_vec(),
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(msg))]
                if msg == &format!("limit {} out of bounds", limit)
        ));
    }

    /// Returns documents (not proof) when prove is false and at least one document
    /// exists; the raw-results execution branch is exercised with actual data.
    #[test]
    fn test_documents_returned_without_proof_when_present() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let data_contract_id = created_data_contract.data_contract().id();
        let document_type_name = "niceDocument";
        let document_type = created_data_contract
            .data_contract()
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let document = document_type
            .random_document(Some(11), platform_version)
            .expect("expected a random doc");

        store_document(
            &platform,
            created_data_contract.data_contract(),
            document_type,
            &document,
            platform_version,
        );

        let request = GetDocumentsRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            limit: 10,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        let Some(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Documents(documents)),
            metadata: Some(_),
        }) = result.data
        else {
            panic!("expected documents, not a proof")
        };
        assert_eq!(documents.documents.len(), 1);
    }

    /// Absent document type should fail even when prove is true (error path is
    /// reached before the proof branch).
    #[test]
    fn test_absent_document_type_with_prove_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let request = GetDocumentsRequestV0 {
            data_contract_id: created_data_contract.data_contract().id().to_vec(),
            document_type: "noSuchTypeInContract".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("document type noSuchTypeInContract not found for contract")
        ));
    }

    /// Invalid start_at (too short) with prove: true must still surface the
    /// InvalidStartsWithClause error rather than short-circuiting to a proof.
    #[test]
    fn test_invalid_start_at_clause_with_prove_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let request = GetDocumentsRequestV0 {
            data_contract_id: created_data_contract.data_contract().id().to_vec(),
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![],
            prove: true,
            start: Some(Start::StartAt(vec![0; 4])), // wrong length
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(msg))]
                if msg == &"start at should be a 32 byte identifier"
        ));
    }

    /// Malformed order_by CBOR must fail even when prove is true.
    #[test]
    fn test_invalid_order_by_with_prove_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let request = GetDocumentsRequestV0 {
            data_contract_id: created_data_contract.data_contract().id().to_vec(),
            document_type: "niceDocument".to_string(),
            r#where: vec![],
            limit: 0,
            order_by: vec![0x9F], // malformed CBOR
            prove: true,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DeserializationError(msg))]
                if msg == "unable to decode 'order_by' query from cbor"
        ));
    }

    /// A where clause that deserializes as valid CBOR but references a field that is
    /// not on any index should be rejected by the drive document-query constructor.
    /// This ensures the `DriveDocumentQuery::from_decomposed_values` error path is
    /// surfaced as a validation error rather than an Err Result.
    #[test]
    fn test_where_clause_on_non_indexed_field_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let created_data_contract = get_data_contract_fixture(None, 0, version.protocol_version);
        store_data_contract(&platform, created_data_contract.data_contract(), version);

        let bogus_clause = drive::query::WhereClause {
            field: "thisFieldIsNotIndexed".to_string(),
            operator: drive::query::WhereOperator::Equal,
            value: Value::Text("value".to_string()),
        };

        let where_cbor =
            serialize_vec_to_cbor(vec![bogus_clause]).expect("should serialize clause to cbor");

        let request = GetDocumentsRequestV0 {
            data_contract_id: created_data_contract.data_contract().id().to_vec(),
            document_type: "niceDocument".to_string(),
            r#where: where_cbor,
            limit: 0,
            order_by: vec![],
            prove: false,
            start: None,
        };

        let result = platform
            .query_documents_v0(request, &state, version)
            .expect("expected query to succeed");

        // Should produce a Query error (validation error, not Err).
        assert!(
            !result.errors.is_empty(),
            "expected an error for invalid where clause"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| matches!(e, QueryError::Query(_))),
            "expected a QueryError::Query variant, got: {:?}",
            result.errors
        );
    }
}
