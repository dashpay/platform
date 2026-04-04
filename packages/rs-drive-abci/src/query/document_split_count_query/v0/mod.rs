use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_split_count_request::GetDocumentsSplitCountRequestV0;
use dapi_grpc::platform::v0::get_documents_split_count_response::{
    get_documents_split_count_response_v0, GetDocumentsSplitCountResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::DriveDocumentQuery;
use drive::util::grove_operations::GroveDBToUse;
use std::collections::BTreeMap;

impl<C> Platform<C> {
    pub(super) fn query_documents_split_count_v0(
        &self,
        GetDocumentsSplitCountRequestV0 {
            data_contract_id,
            document_type: document_type_name,
            r#where,
            split_count_by_index_property,
            prove,
        }: GetDocumentsSplitCountRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsSplitCountResponseV0>, Error> {
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

        // Validate the split property exists in the document type
        if split_count_by_index_property.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(
                    "split_count_by_index_property must not be empty".to_string(),
                ),
            ));
        }

        // Check that the property exists in the document type schema
        if document_type
            .properties()
            .get(split_count_by_index_property.as_str())
            .is_none()
        {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "property {} not found in document type {}",
                    split_count_by_index_property, document_type_name
                )),
            ));
        }

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

        // Remove the limit so we count ALL matching documents, not just up to the
        // default query limit. A split count query needs to return complete counts
        // across all values of the split property.
        drive_query.limit = None;

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

            GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::Proof(proof)),
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

            // Deserialize documents and split count by the specified property
            let mut counts_by_key: BTreeMap<Vec<u8>, u64> = BTreeMap::new();

            for raw_document in &results {
                let document =
                    check_validation_result_with_data!(dpp::document::Document::from_bytes(
                        raw_document.as_slice(),
                        document_type,
                        platform_version,
                    )
                    .map_err(|e| QueryError::InvalidArgument(format!(
                        "failed to deserialize document: {}",
                        e
                    ))));

                let key = if let Some(value) =
                    document.properties().get(&split_count_by_index_property)
                {
                    // Serialize the property value to CBOR bytes for the key
                    value.to_cbor_buffer().unwrap_or_default()
                } else {
                    // Null / missing key
                    Vec::new()
                };

                *counts_by_key.entry(key).or_insert(0) += 1;
            }

            let entries = counts_by_key
                .into_iter()
                .map(
                    |(key, count)| get_documents_split_count_response_v0::SplitCountEntry {
                        key,
                        count,
                    },
                )
                .collect();

            GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::SplitCounts(
                    get_documents_split_count_response_v0::SplitCounts { entries },
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
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_documents_split_count_no_prove() {
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

        let mut std_rng = StdRng::seed_from_u64(600);
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "firstName".to_string(),
            prove: false,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        match result.data {
            Some(GetDocumentsSplitCountResponseV0 {
                result:
                    Some(get_documents_split_count_response_v0::Result::SplitCounts(split_counts)),
                metadata: Some(_),
            }) => {
                // The total count across all splits should equal 5
                let total: u64 = split_counts.entries.iter().map(|e| e.count).sum();
                assert_eq!(total, 5, "expected total split count of 5 documents");
                // Each entry should have a non-empty key (firstName is required)
                for entry in &split_counts.entries {
                    assert!(!entry.key.is_empty(), "expected non-empty split key");
                    assert!(entry.count > 0, "expected positive count per split");
                }
            }
            other => panic!("expected split counts result, got {:?}", other),
        }
    }

    #[test]
    fn test_documents_split_count_with_prove() {
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

        let mut std_rng = StdRng::seed_from_u64(600);
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "firstName".to_string(),
            prove: true,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        assert!(matches!(
            result.data,
            Some(GetDocumentsSplitCountResponseV0 {
                result: Some(get_documents_split_count_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_documents_split_count_empty_split_property() {
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "".to_string(),
            prove: false,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == "split_count_by_index_property must not be empty"
        ));
    }

    #[test]
    fn test_documents_split_count_nonexistent_property() {
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

        let request = GetDocumentsSplitCountRequestV0 {
            data_contract_id: data_contract_id.to_vec(),
            document_type: document_type_name.to_string(),
            r#where: vec![],
            split_count_by_index_property: "nonExistentProp".to_string(),
            prove: false,
        };

        let result = platform
            .query_documents_split_count_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("property nonExistentProp not found")
        ));
    }
}
