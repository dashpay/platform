use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
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
use drive::query::DriveDocumentQuery;

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

        let where_clause = if r#where.is_empty() {
            Value::Null
        } else {
            check_validation_result_with_data!(ciborium::de::from_reader(r#where.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'where' query from cbor".to_string(),
                    ))
                }))
        };

        let order_by = if !order_by.is_empty() {
            check_validation_result_with_data!(ciborium::de::from_reader(order_by.as_slice())
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode 'order_by' query from cbor".to_string(),
                    ))
                }))
        } else {
            None
        };

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

        if limit > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!("limit {} out of bounds", limit)),
            )));
        }

        let drive_query =
            check_validation_result_with_data!(DriveDocumentQuery::from_decomposed_values(
                where_clause,
                order_by,
                Some(if limit == 0 {
                    self.config.drive.default_query_limit
                } else {
                    limit as u16
                }),
                start_at,
                start_at_included,
                None,
                contract_ref,
                document_type,
                &self.config.drive,
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

            GetDocumentsResponseV0 {
                result: Some(get_documents_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
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
            .load_withdrawals();

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
                in_clause: None,
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
            .load_withdrawals();

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
                in_clause: None,
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
            .load_withdrawals();

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
                in_clause: Some(WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::U8(0),
                        Value::U8(1),
                        Value::U8(2),
                        Value::U8(3),
                        Value::U8(4),
                    ]),
                }),
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
                .in_clause
                .clone()
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
            .load_withdrawals();

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
                in_clause: Some(WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::I64(0),
                        Value::I64(1),
                        Value::I64(2),
                        Value::I64(3),
                        Value::I64(4),
                    ]),
                }),
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
                .in_clause
                .clone()
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
            .load_withdrawals();

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
                in_clause: Some(WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::I64(0),
                        Value::I64(1),
                        Value::I64(2),
                        Value::I64(3),
                        Value::I64(4),
                    ]),
                }),
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
                .in_clause
                .clone()
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
}
