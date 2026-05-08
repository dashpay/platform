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
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{DriveDocumentCountQuery, DriveDocumentQuery, WhereClause};
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_documents_count_v0(
        &self,
        GetDocumentsCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
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

        let response = if prove {
            // For prove path, use the standard DriveDocumentQuery approach.
            // We still need the full path query structure for proof generation.
            let mut drive_query =
                check_validation_result_with_data!(DriveDocumentQuery::from_decomposed_values(
                    where_clause,
                    None,
                    Some(self.config.drive.default_query_limit),
                    None,
                    true,
                    None,
                    contract_ref,
                    document_type,
                    &self.config.drive,
                ));

            // Cap the proof at u16::MAX matching documents. The proof
            // verifier returns the count by deserializing every document in
            // the proof, so an unbounded query would force the server to
            // materialize and the client to verify an arbitrarily large set
            // of documents purely to learn their count. Until count-tree
            // proofs are implemented, callers that need exact counts on
            // larger result sets should use `prove=false` with a covering
            // countable index.
            drive_query.limit = Some(u16::MAX);

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

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            // The count fast path supports only Equal and In where-clause
            // operators. Range operators (>, <, between, startsWith) need a
            // boundary walk that the current count-tree path query model
            // cannot express; surface that as a clear error rather than
            // letting it fall through and silently drop the predicate.
            if DriveDocumentCountQuery::has_unsupported_operator(&all_where_clauses) {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "count query supports only `==` and `in` where-clause operators; \
                         range operators (`>`, `<`, `between`, `startsWith`) are not yet \
                         supported on the no-prove path"
                            .to_string(),
                    ),
                ));
            }

            // For no-prove path, use CountTree-based O(1) counting when possible.
            // Find a countable index that matches the where clause properties.
            let countable_index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                document_type.indexes(),
                &all_where_clauses,
            );

            let count = if let Some(index) = countable_index {
                let count_query = DriveDocumentCountQuery {
                    document_type,
                    contract_id: contract_id.to_buffer(),
                    document_type_name: document_type_name.clone(),
                    index,
                    where_clauses: all_where_clauses,
                    split_by_property: None,
                };

                let results = count_query.execute_no_proof(&self.drive, None, platform_version)?;

                // For a total count query, execute_no_proof returns a single entry
                // with an empty key and the total count.
                results.first().map_or(0, |entry| entry.count)
            } else {
                // No countable index found. Return an error telling the caller
                // that count queries require a countable index.
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(
                        "count query requires a countable index on the document type that \
                         matches the where clause properties"
                            .to_string(),
                    ),
                ));
            };

            GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 5, "expected count of 5 documents");
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
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 0, "expected count of 0 documents");
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
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsCountResponseV0 {
                result: Some(get_documents_count_response_v0::Result::Count(count)),
                metadata: Some(_),
            }) => {
                assert_eq!(count, 5, "expected count of 5 (3 age=30 + 2 age=40)");
            }
            other => panic!("expected count result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_count_rejects_range_operator() {
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

        // [["age", ">", 20]] — range operator, must be rejected on no-prove path.
        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text(">".to_string()),
            Value::U64(20),
        ])];

        let request = GetDocumentsCountRequestV0 {
            data_contract_id: data_contract.id().to_vec(),
            document_type: "person".to_string(),
            r#where: serialize_where_clauses_to_cbor(where_clauses),
            prove: false,
        };

        let result = platform
            .query_documents_count_v0(request, &state, version)
            .expect("expected query to return validation error");

        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("range operators") && msg.contains("not yet")
            ),
            "expected range-operator rejection, got {:?}",
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
}
