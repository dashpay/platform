use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_count_request::GetDocumentsCountRequestV0;
use dapi_grpc::platform::v0::get_documents_count_response::{
    get_documents_count_response_v0, GetDocumentsCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{DocumentCountRequest, DocumentCountResponse, SplitCountEntry, WhereClause};
use drive::util::grove_operations::GroveDBToUse;

/// Wrap a vector of [`SplitCountEntry`]s plus current-state metadata
/// into the protobuf `GetDocumentsCountResponseV0`. Pulled out as a
/// free function so the per-mode match arms in
/// [`Platform::query_documents_count_v0`] can each be a single
/// expression instead of inlining the same shape three times.
fn count_response_with_entries<C>(
    entries: Vec<SplitCountEntry>,
    platform: &Platform<C>,
    platform_state: &PlatformState,
) -> GetDocumentsCountResponseV0 {
    let entries: Vec<get_documents_count_response_v0::CountEntry> = entries
        .into_iter()
        .map(|e| get_documents_count_response_v0::CountEntry {
            key: e.key,
            count: e.count,
        })
        .collect();
    GetDocumentsCountResponseV0 {
        result: Some(get_documents_count_response_v0::Result::Counts(
            get_documents_count_response_v0::CountResults { entries },
        )),
        metadata: Some(platform.response_metadata_v0(platform_state, CheckpointUsed::Current)),
    }
}

impl<C> Platform<C> {
    pub(super) fn query_documents_count_v0(
        &self,
        GetDocumentsCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            return_distinct_counts_in_range,
            order_by_ascending,
            limit,
            start_after_split_key,
            prove,
        }: GetDocumentsCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsCountResponseV0>, Error> {
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

        // Parse where clauses into WhereClause structs so we can match them against
        // index properties for the CountTree path.
        let all_where_clauses: Vec<WhereClause> =
            check_validation_result_with_data!(match &where_clause {
                Value::Null => Ok(vec![]),
                Value::Array(clauses) => clauses
                    .iter()
                    .map(|wc| {
                        if let Value::Array(components) = wc {
                            WhereClause::from_components(components).map_err(|e| match e {
                                drive::error::Error::Query(qe) => QueryError::Query(qe),
                                other => QueryError::InvalidArgument(format!(
                                    "error parsing where clauses: {}",
                                    other
                                )),
                            })
                        } else {
                            Err(QueryError::Query(
                                QuerySyntaxError::InvalidFormatWhereClause(
                                    "where clause must be an array",
                                ),
                            ))
                        }
                    })
                    .collect::<Result<Vec<WhereClause>, QueryError>>(),
                _ => Err(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
                )),
            });

        // Single rs-drive call owns mode detection, index picking, and
        // per-mode dispatch. The handler is left with: build request,
        // pre-clamp limit, map drive result to protobuf response.
        let request = DocumentCountRequest {
            contract: contract_ref,
            document_type,
            where_clauses: all_where_clauses,
            raw_where_value: where_clause,
            return_distinct_counts_in_range,
            order_by_ascending,
            // Server-side limit clamp: clients may request more than
            // the configured ceiling but the server enforces it.
            limit: limit.map(|req| req.min(self.config.drive.max_query_limit as u32)),
            start_after_split_key,
            prove,
            drive_config: &self.config.drive,
        };
        let drive_response =
            match self
                .drive
                .execute_document_count_request(request, None, platform_version)
            {
                Ok(r) => r,
                Err(drive::error::Error::Query(qe)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(qe)));
                }
                Err(e) => return Err(e.into()),
            };

        let response = match drive_response {
            DocumentCountResponse::Counts(entries) => {
                count_response_with_entries(entries, self, platform_state)
            }
            DocumentCountResponse::Proof(proof_bytes) => {
                let (grovedb_used, proof) =
                    self.response_proof_v0(platform_state, proof_bytes, GroveDBToUse::Current)?;
                GetDocumentsCountResponseV0 {
                    result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                    metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
                }
            }
        };
        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_documents_count_no_prove() {
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

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..5 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                metadata: Some(_),
            }) => {
                let total: u64 = counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected count of 5 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_empty_result() {
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

        let data_contract_id = data_contract.id();
        let document_type_name = "person";

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                metadata: Some(_),
            }) => {
                let total: u64 = counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 0, "expected count of 0 documents");
            }
            other => panic!("expected count result, got {:?}", other),
        }
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

    #[test]
    fn test_documents_count_with_in_operator() {
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

        // 3 docs with age=30, 2 with age=40, 1 with age=50.
        store_person_document(
            &platform,
            &data_contract,
            [1u8; 32],
            "Alice",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [2u8; 32],
            "Bob",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [3u8; 32],
            "Carol",
            "Smith",
            30,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [4u8; 32],
            "Dave",
            "Smith",
            40,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [5u8; 32],
            "Eve",
            "Smith",
            40,
            platform_version,
        );
        store_person_document(
            &platform,
            &data_contract,
            [6u8; 32],
            "Frank",
            "Smith",
            50,
            platform_version,
        );

        // [["age", "in", [30, 40]]]
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                metadata: Some(_),
            }) => {
                let total: u64 = counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected count of 5 (3 age=30 + 2 age=40)");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_range_without_range_countable_index_returns_clear_error() {
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

        // [["age", ">", 20]] — range operator on a contract whose `age`
        // index is `countable` but NOT `range_countable`. The range
        // path now accepts range operators, but the picker must report
        // "no usable index" so the handler surfaces a clear error.
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text(">".to_string()),
            Value::U64(20),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to return validation error");

        // Step 2 of the refactor moved the no-covering-index check into
        // rs-drive, where it surfaces as
        // `Query(WhereClauseOnNonIndexedProperty)` rather than the
        // handler-local `InvalidArgument`. Both shapes are valid
        // rejections — accept either.
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

    #[test]
    fn test_documents_count_with_prove() {
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

        let data_contract_id = data_contract.id();
        let document_type_name = "person";
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for _ in 0..3 {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &data_contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert!(matches!(
            result.data,
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    /// End-to-end test for the range count happy path against a v12
    /// contract whose `widget` document type carries a
    /// `rangeCountable: true` index over `color`. Exercises the
    /// `find_range_countable_index_for_where_clauses` →
    /// `execute_range_count_no_proof` route in the no-prove handler,
    /// in both summed and distinct modes plus the pagination knobs.
    #[test]
    fn test_documents_count_range_query_no_prove() {
        use dpp::data_contract::DataContractFactory;
        use dpp::document::DocumentV0Setters;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        // Build an in-memory v12 contract with a range_countable index.
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

        // 6 docs across 3 colors: red×2, blue×1, green×3.
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

        // Helper: issue a range count request with the given options.
        let make_request = |distinct: bool, limit: Option<u32>, ascending: Option<bool>| {
            let where_clauses = vec![Value::Array(vec![
                Value::Text("color".to_string()),
                Value::Text(">".to_string()),
                Value::Text("blue".to_string()),
            ])];
            GetDocumentsCountRequestV0 {
                data_contract_id: contract.id().to_vec(),
                document_type: "widget".to_string(),
                r#where: serialize_where_clauses_to_cbor(where_clauses),
                return_distinct_counts_in_range: distinct,
                order_by_ascending: ascending,
                limit,
                start_after_split_key: None,
                prove: false,
            }
        };

        // Sum mode: green(3) + red(2) = 5.
        let result = platform
            .query_documents_count_v0(make_request(false, None, None), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                ..
            }) => {
                assert_eq!(counts.entries.len(), 1, "summed mode → one entry");
                assert!(counts.entries[0].key.is_empty());
                assert_eq!(counts.entries[0].count, 5);
            }
            other => panic!("expected counts result, got {:?}", other),
        }

        // Distinct mode ascending: [(green, 3), (red, 2)].
        let result = platform
            .query_documents_count_v0(make_request(true, None, Some(true)), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                ..
            }) => {
                assert_eq!(counts.entries.len(), 2);
                assert_eq!(counts.entries[0].key, b"green".to_vec());
                assert_eq!(counts.entries[0].count, 3);
                assert_eq!(counts.entries[1].key, b"red".to_vec());
                assert_eq!(counts.entries[1].count, 2);
            }
            other => panic!("expected counts result, got {:?}", other),
        }

        // Distinct mode with limit=1: only the first entry (ascending → green).
        let result = platform
            .query_documents_count_v0(make_request(true, Some(1), Some(true)), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                ..
            }) => {
                assert_eq!(counts.entries.len(), 1);
                assert_eq!(counts.entries[0].key, b"green".to_vec());
            }
            other => panic!("expected counts result, got {:?}", other),
        }

        // Distinct descending: [(red, 2), (green, 3)].
        let result = platform
            .query_documents_count_v0(make_request(true, None, Some(false)), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Counts(counts)),
                ..
            }) => {
                assert_eq!(counts.entries.len(), 2);
                assert_eq!(counts.entries[0].key, b"red".to_vec());
                assert_eq!(counts.entries[1].key, b"green".to_vec());
            }
            other => panic!("expected counts result, got {:?}", other),
        }
    }

    /// `return_distinct_counts_in_range = true` is rejected on the
    /// prove path because grovedb's `AggregateCountOnRange` proof
    /// returns one aggregate, not per-distinct-value entries.
    #[test]
    fn test_documents_count_range_with_prove_rejects_distinct() {
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

        let where_clauses = vec![Value::Array(vec![
            Value::Text("color".to_string()),
            Value::Text(">".to_string()),
            Value::Text("blue".to_string()),
        ])];
        let request = GetDocumentsCountRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            return_distinct_counts_in_range: true,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
            prove: true,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("query should return validation error");
        let _ = platform_version;
        // After the detect_mode refactor this rejection now comes from
        // rs-drive's where-clause validation rather than an inline
        // handler check, so it surfaces as a `Query(InvalidWhereClauseComponents)`
        // rather than `InvalidArgument`. Both shape variants are valid
        // rejections; we accept either.
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("return_distinct_counts_in_range")
            ) || matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::InvalidWhereClauseComponents(msg))]
                    if msg.contains("return_distinct_counts_in_range")
            ),
            "expected return_distinct_counts_in_range rejection on prove path, got {:?}",
            result.errors
        );
    }
}
